// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Functions to procedurally construct contract code used for benchmarking.
//!
//! In order to be able to benchmark events that are triggered by contract execution
//! (API calls into seal, individual instructions), we need to generate contracts that
//! perform those events. Because those contracts can get very big we cannot simply define
//! them as text (.wat) as this will be too slow and consume too much memory. Therefore
//! we define this simple definition of a contract that can be passed to `create_code` that
//! compiles it down into a `WasmModule` that can be used as a contract's code.

use crate::Config;
use alloc::{borrow::ToOwned, vec, vec::Vec};
use frame_support::traits::Get;
use sp_runtime::{traits::Hash, Saturating};
use wasm_instrument::parity_wasm::{
	builder,
	elements::{
		self, BlockType, CustomSection, FuncBody, Instruction, Instructions, Local, Section,
		ValueType,
	},
};

/// The location where to put the generated code.
pub enum Location {
	/// Generate all code into the `call` exported function.
	Call,
	/// Generate all code into the `deploy` exported function.
	Deploy,
}

/// Pass to `create_code` in order to create a compiled `WasmModule`.
///
/// This exists to have a more declarative way to describe a wasm module than to use
/// parity-wasm directly. It is tailored to fit the structure of contracts that are
/// needed for benchmarking.
#[derive(Default)]
pub struct ModuleDefinition {
	/// Imported memory attached to the module. No memory is imported if `None`.
	pub memory: Option<ImportedMemory>,
	/// Initializers for the imported memory.
	pub data_segments: Vec<DataSegment>,
	/// Creates the supplied amount of i64 mutable globals initialized with random values.
	pub num_globals: u32,
	/// List of functions that the module should import. They start with index 0.
	pub imported_functions: Vec<ImportedFunction>,
	/// Function body of the exported `deploy` function. Body is empty if `None`.
	/// Its index is `imported_functions.len()`.
	pub deploy_body: Option<FuncBody>,
	/// Function body of the exported `call` function. Body is empty if `None`.
	/// Its index is `imported_functions.len() + 1`.
	pub call_body: Option<FuncBody>,
	/// Function body of a non-exported function with index `imported_functions.len() + 2`.
	pub aux_body: Option<FuncBody>,
	/// The amount of I64 arguments the aux function should have.
	pub aux_arg_num: u32,
	/// Create a table containing function pointers.
	pub table: Option<TableSegment>,
	/// Create a section named "dummy" of the specified size. This is useful in order to
	/// benchmark the overhead of loading and storing codes of specified sizes. The dummy
	/// section only contributes to the size of the contract but does not affect execution.
	pub dummy_section: u32,
}

pub struct TableSegment {
	/// How many elements should be created inside the table.
	pub num_elements: u32,
	/// The function index with which all table elements should be initialized.
	pub function_index: u32,
}

pub struct DataSegment {
	pub offset: u32,
	pub value: Vec<u8>,
}

#[derive(Clone)]
pub struct ImportedMemory {
	pub min_pages: u32,
	pub max_pages: u32,
}

impl ImportedMemory {
	pub fn max<T: Config>() -> Self {
		let pages = max_pages::<T>();
		Self { min_pages: pages, max_pages: pages }
	}
}

pub struct ImportedFunction {
	pub module: &'static str,
	pub name: &'static str,
	pub params: Vec<ValueType>,
	pub return_type: Option<ValueType>,
}

/// A wasm module ready to be put on chain.
#[derive(Clone)]
pub struct WasmModule<T: Config> {
	pub code: Vec<u8>,
	pub hash: <T::Hashing as Hash>::Output,
}

impl<T: Config> From<ModuleDefinition> for WasmModule<T> {
	fn from(def: ModuleDefinition) -> Self {
		// internal functions start at that offset.
		let func_offset = u32::try_from(def.imported_functions.len()).unwrap();

		// Every contract must export "deploy" and "call" functions.
		let mut contract = builder::module()
			// deploy function (first internal function)
			.function()
			.signature()
			.build()
			.with_body(
				def.deploy_body
					.unwrap_or_else(|| FuncBody::new(Vec::new(), Instructions::empty())),
			)
			.build()
			// call function (second internal function)
			.function()
			.signature()
			.build()
			.with_body(
				def.call_body
					.unwrap_or_else(|| FuncBody::new(Vec::new(), Instructions::empty())),
			)
			.build()
			.export()
			.field("deploy")
			.internal()
			.func(func_offset)
			.build()
			.export()
			.field("call")
			.internal()
			.func(func_offset + 1)
			.build();

		// If specified we add an additional internal function
		if let Some(body) = def.aux_body {
			let mut signature = contract.function().signature();
			for _ in 0..def.aux_arg_num {
				signature = signature.with_param(ValueType::I64);
			}
			contract = signature.build().with_body(body).build();
		}

		// Grant access to linear memory.
		// Every contract module is required to have an imported memory.
		// If no memory is specified in the passed ModuleDefinition, then
		// default to (1, 1).
		let (init, max) = if let Some(memory) = &def.memory {
			(memory.min_pages, Some(memory.max_pages))
		} else {
			(1, Some(1))
		};

		contract = contract.import().path("env", "memory").external().memory(init, max).build();

		// Import supervisor functions. They start with idx 0.
		for func in def.imported_functions {
			let sig = builder::signature()
				.with_params(func.params)
				.with_results(func.return_type)
				.build_sig();
			let sig = contract.push_signature(sig);
			contract = contract
				.import()
				.module(func.module)
				.field(func.name)
				.with_external(elements::External::Function(sig))
				.build();
		}

		// Initialize memory
		for data in def.data_segments {
			contract = contract
				.data()
				.offset(Instruction::I32Const(data.offset as i32))
				.value(data.value)
				.build()
		}

		// Add global variables
		if def.num_globals > 0 {
			use rand::{distributions::Standard, prelude::*};
			let rng = rand_pcg::Pcg32::seed_from_u64(3112244599778833558);
			for val in rng.sample_iter(Standard).take(def.num_globals as usize) {
				contract = contract
					.global()
					.value_type()
					.i64()
					.mutable()
					.init_expr(Instruction::I64Const(val))
					.build()
			}
		}

		// Add function pointer table
		if let Some(table) = def.table {
			contract = contract
				.table()
				.with_min(table.num_elements)
				.with_max(Some(table.num_elements))
				.with_element(0, vec![table.function_index; table.num_elements as usize])
				.build();
		}

		// Add the dummy section
		if def.dummy_section > 0 {
			contract = contract.with_section(Section::Custom(CustomSection::new(
				"dummy".to_owned(),
				vec![42; def.dummy_section as usize],
			)));
		}

		let code = contract.build().into_bytes().unwrap();
		let hash = T::Hashing::hash(&code);
		Self { code: code.into(), hash }
	}
}

impl<T: Config> WasmModule<T> {
	/// Creates a wasm module with an empty `call` and `deploy` function and nothing else.
	pub fn dummy() -> Self {
		ModuleDefinition::default().into()
	}

	/// Same as `dummy` but with maximum sized linear memory and a dummy section of specified size.
	pub fn dummy_with_bytes(dummy_bytes: u32) -> Self {
		// We want the module to have the size `dummy_bytes`.
		// This is not completely correct as the overhead grows when the contract grows
		// because of variable length integer encoding. However, it is good enough to be that
		// close for benchmarking purposes.
		let module_overhead = 65;
		ModuleDefinition {
			memory: Some(ImportedMemory::max::<T>()),
			dummy_section: dummy_bytes.saturating_sub(module_overhead),
			..Default::default()
		}
		.into()
	}

	/// Creates a wasm module of `target_bytes` size. Used to benchmark the performance of
	/// `instantiate_with_code` for different sizes of wasm modules. The generated module maximizes
	/// instrumentation runtime by nesting blocks as deeply as possible given the byte budget.
	/// `code_location`: Whether to place the code into `deploy` or `call`.
	pub fn sized(target_bytes: u32, code_location: Location, use_float: bool) -> Self {
		use self::elements::Instruction::{End, GetLocal, If, Return};
		// Base size of a contract is 63 bytes and each expansion adds 6 bytes.
		// We do one expansion less to account for the code section and function body
		// size fields inside the binary wasm module representation which are leb128 encoded
		// and therefore grow in size when the contract grows. We are not allowed to overshoot
		// because of the maximum code size that is enforced by `instantiate_with_code`.
		let mut expansions = (target_bytes.saturating_sub(63) / 6).saturating_sub(1);
		const EXPANSION: [Instruction; 4] = [GetLocal(0), If(BlockType::NoResult), Return, End];
		let mut locals = vec![Local::new(1, ValueType::I32)];
		if use_float {
			locals.push(Local::new(1, ValueType::F32));
			locals.push(Local::new(2, ValueType::F32));
			locals.push(Local::new(3, ValueType::F32));
			expansions.saturating_dec();
		}
		let mut module =
			ModuleDefinition { memory: Some(ImportedMemory::max::<T>()), ..Default::default() };
		let body = Some(body::repeated_with_locals(&locals, expansions, &EXPANSION));
		match code_location {
			Location::Call => module.call_body = body,
			Location::Deploy => module.deploy_body = body,
		}
		module.into()
	}

	/// Creates a wasm module that calls the imported function `noop` `repeat` times.
	pub fn noop(repeat: u32) -> Self {
		let pages = max_pages::<T>();
		ModuleDefinition {
			memory: Some(ImportedMemory::max::<T>()),
			imported_functions: vec![ImportedFunction {
				module: "seal0",
				name: "noop",
				params: vec![],
				return_type: None,
			}],
			// Write the output buffer size. The output size will be overwritten by the
			// supervisor with the real size when calling the getter. Since this size does not
			// change between calls it suffices to start with an initial value and then just
			// leave as whatever value was written there.
			data_segments: vec![DataSegment {
				offset: 0,
				value: (pages * 64 * 1024 - 4).to_le_bytes().to_vec(),
			}],
			call_body: Some(body::repeated(
				repeat,
				&[
					Instruction::Call(0), // call the imported function
				],
			)),
			..Default::default()
		}
		.into()
	}
}

/// Mechanisms to generate a function body that can be used inside a `ModuleDefinition`.
pub mod body {
	use super::*;

	pub fn repeated(repetitions: u32, instructions: &[Instruction]) -> FuncBody {
		repeated_with_locals(&[], repetitions, instructions)
	}

	pub fn repeated_with_locals(
		locals: &[Local],
		repetitions: u32,
		instructions: &[Instruction],
	) -> FuncBody {
		let instructions = Instructions::new(
			instructions
				.iter()
				.cycle()
				.take(instructions.len() * usize::try_from(repetitions).unwrap())
				.cloned()
				.chain(core::iter::once(Instruction::End))
				.collect(),
		);
		FuncBody::new(locals.to_vec(), instructions)
	}

	pub fn repeated_with_locals_using<const N: usize>(
		locals: &[Local],
		repetitions: u32,
		mut f: impl FnMut() -> [Instruction; N],
	) -> FuncBody {
		let mut instructions = Vec::new();
		for _ in 0..repetitions {
			instructions.extend(f());
		}
		instructions.push(Instruction::End);
		FuncBody::new(locals.to_vec(), Instructions::new(instructions))
	}
}

/// The maximum amount of pages any contract is allowed to have according to the current `Schedule`.
pub fn max_pages<T: Config>() -> u32 {
	T::Schedule::get().limits.memory_pages
}
