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

//! Inherents for BABE

use sp_inherents::{Error, InherentData, InherentIdentifier};

/// The BABE inherent identifier.
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"babeslot";

/// The type of the BABE inherent.
pub type InherentType = sp_consensus_slots::Slot;

/// Create inherent data providers for BABE with timestamp.
#[cfg(feature = "std")]
pub type BabeCreateInherentDataProviders<Block> = std::sync::Arc<
	dyn sp_inherents::CreateInherentDataProviders<
		Block,
		(),
		InherentDataProviders = (InherentDataProvider, sp_timestamp::InherentDataProvider),
	>,
>;

/// Auxiliary trait to extract BABE inherent data.
pub trait BabeInherentData {
	/// Get BABE inherent data.
	fn babe_inherent_data(&self) -> Result<Option<InherentType>, Error>;
	/// Replace BABE inherent data.
	fn babe_replace_inherent_data(&mut self, new: InherentType);
}

impl BabeInherentData for InherentData {
	fn babe_inherent_data(&self) -> Result<Option<InherentType>, Error> {
		self.get_data(&INHERENT_IDENTIFIER)
	}

	fn babe_replace_inherent_data(&mut self, new: InherentType) {
		self.replace_data(INHERENT_IDENTIFIER, &new);
	}
}

/// Provides the slot duration inherent data for BABE.
// TODO: Remove in the future. https://github.com/paritytech/substrate/issues/8029
#[cfg(feature = "std")]
pub struct InherentDataProvider {
	slot: InherentType,
}

#[cfg(feature = "std")]
impl InherentDataProvider {
	/// Create new inherent data provider from the given `slot`.
	pub fn new(slot: InherentType) -> Self {
		Self { slot }
	}

	/// Creates the inherent data provider by calculating the slot from the given
	/// `timestamp` and `duration`.
	pub fn from_timestamp_and_slot_duration(
		timestamp: sp_timestamp::Timestamp,
		slot_duration: sp_consensus_slots::SlotDuration,
	) -> Self {
		let slot = InherentType::from_timestamp(timestamp, slot_duration);

		Self { slot }
	}

	/// Returns the `slot` of this inherent data provider.
	pub fn slot(&self) -> InherentType {
		self.slot
	}
}

#[cfg(feature = "std")]
impl core::ops::Deref for InherentDataProvider {
	type Target = InherentType;

	fn deref(&self) -> &Self::Target {
		&self.slot
	}
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
impl sp_inherents::InherentDataProvider for InherentDataProvider {
	async fn provide_inherent_data(&self, inherent_data: &mut InherentData) -> Result<(), Error> {
		inherent_data.put_data(INHERENT_IDENTIFIER, &self.slot)
	}

	async fn try_handle_error(
		&self,
		_: &InherentIdentifier,
		_: &[u8],
	) -> Option<Result<(), Error>> {
		// There is no error anymore
		None
	}
}
