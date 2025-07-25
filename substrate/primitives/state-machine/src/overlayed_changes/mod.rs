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

//! The overlayed changes to state.

mod changeset;
mod offchain;

use self::changeset::OverlayedChangeSet;
use crate::{backend::Backend, stats::StateMachineStats, BackendTransaction, DefaultError};
use alloc::{collections::btree_set::BTreeSet, vec::Vec};
use codec::{Decode, Encode};
use hash_db::Hasher;
pub use offchain::OffchainOverlayedChanges;
use sp_core::{
	offchain::OffchainOverlayedChange,
	storage::{well_known_keys::EXTRINSIC_INDEX, ChildInfo, StateVersion},
};
#[cfg(feature = "std")]
use sp_externalities::{Extension, Extensions};
use sp_trie::{empty_child_trie_root, LayoutV1};

#[cfg(not(feature = "std"))]
use alloc::collections::btree_map::BTreeMap as Map;
#[cfg(feature = "std")]
use std::collections::{hash_map::Entry as MapEntry, HashMap as Map};

#[cfg(feature = "std")]
use std::{
	any::{Any, TypeId},
	boxed::Box,
};

pub use self::changeset::{AlreadyInRuntime, NoOpenTransaction, NotInRuntime, OverlayedValue};

/// Changes that are made outside of extrinsics are marked with this index;
pub const NO_EXTRINSIC_INDEX: u32 = 0xffffffff;

/// Storage key.
pub type StorageKey = Vec<u8>;

/// Storage value.
pub type StorageValue = Vec<u8>;

/// In memory array of storage values.
pub type StorageCollection = Vec<(StorageKey, Option<StorageValue>)>;

/// In memory arrays of storage values for multiple child tries.
pub type ChildStorageCollection = Vec<(StorageKey, StorageCollection)>;

/// In memory array of storage values.
pub type OffchainChangesCollection = Vec<((Vec<u8>, Vec<u8>), OffchainOverlayedChange)>;

/// Keep trace of extrinsics index for a modified value.
#[derive(Debug, Default, Eq, PartialEq, Clone)]
pub struct Extrinsics(Vec<u32>);

impl Extrinsics {
	/// Extracts extrinsics into a `BTreeSets`.
	fn copy_extrinsics_into(&self, dest: &mut BTreeSet<u32>) {
		dest.extend(self.0.iter())
	}

	/// Add an extrinsics.
	fn insert(&mut self, ext: u32) {
		if Some(&ext) != self.0.last() {
			self.0.push(ext);
		}
	}

	/// Extend `self` with `other`.
	fn extend(&mut self, other: Self) {
		self.0.extend(other.0.into_iter());
	}
}

/// The set of changes that are overlaid onto the backend.
///
/// It allows changes to be modified using nestable transactions.
pub struct OverlayedChanges<H: Hasher> {
	/// Top level storage changes.
	top: OverlayedChangeSet,
	/// Child storage changes. The map key is the child storage key without the common prefix.
	children: Map<StorageKey, (OverlayedChangeSet, ChildInfo)>,
	/// Offchain related changes.
	offchain: OffchainOverlayedChanges,
	/// Transaction index changes,
	transaction_index_ops: Vec<IndexOperation>,
	/// True if extrinsics stats must be collected.
	collect_extrinsics: bool,
	/// Collect statistic on this execution.
	stats: StateMachineStats,
	/// Caches the "storage transaction" that is created while calling `storage_root`.
	///
	/// This transaction can be applied to the backend to persist the state changes.
	storage_transaction_cache: Option<StorageTransactionCache<H>>,
}

impl<H: Hasher> Default for OverlayedChanges<H> {
	fn default() -> Self {
		Self {
			top: Default::default(),
			children: Default::default(),
			offchain: Default::default(),
			transaction_index_ops: Default::default(),
			collect_extrinsics: Default::default(),
			stats: Default::default(),
			storage_transaction_cache: None,
		}
	}
}

impl<H: Hasher> Clone for OverlayedChanges<H> {
	fn clone(&self) -> Self {
		Self {
			top: self.top.clone(),
			children: self.children.clone(),
			offchain: self.offchain.clone(),
			transaction_index_ops: self.transaction_index_ops.clone(),
			collect_extrinsics: self.collect_extrinsics,
			stats: self.stats.clone(),
			storage_transaction_cache: self.storage_transaction_cache.clone(),
		}
	}
}

impl<H: Hasher> core::fmt::Debug for OverlayedChanges<H> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("OverlayedChanges")
			.field("top", &self.top)
			.field("children", &self.children)
			.field("offchain", &self.offchain)
			.field("transaction_index_ops", &self.transaction_index_ops)
			.field("collect_extrinsics", &self.collect_extrinsics)
			.field("stats", &self.stats)
			.field("storage_transaction_cache", &self.storage_transaction_cache)
			.finish()
	}
}

/// Transaction index operation.
#[derive(Debug, Clone)]
pub enum IndexOperation {
	/// Insert transaction into index.
	Insert {
		/// Extrinsic index in the current block.
		extrinsic: u32,
		/// Data content hash.
		hash: Vec<u8>,
		/// Indexed data size.
		size: u32,
	},
	/// Renew existing transaction storage.
	Renew {
		/// Extrinsic index in the current block.
		extrinsic: u32,
		/// Referenced index hash.
		hash: Vec<u8>,
	},
}

/// A storage changes structure that can be generated by the data collected in [`OverlayedChanges`].
///
/// This contains all the changes to the storage and transactions to apply theses changes to the
/// backend.
pub struct StorageChanges<H: Hasher> {
	/// All changes to the main storage.
	///
	/// A value of `None` means that it was deleted.
	pub main_storage_changes: StorageCollection,
	/// All changes to the child storages.
	pub child_storage_changes: ChildStorageCollection,
	/// Offchain state changes to write to the offchain database.
	pub offchain_storage_changes: OffchainChangesCollection,
	/// A transaction for the backend that contains all changes from
	/// [`main_storage_changes`](StorageChanges::main_storage_changes) and from
	/// [`child_storage_changes`](StorageChanges::child_storage_changes).
	/// [`offchain_storage_changes`](StorageChanges::offchain_storage_changes).
	pub transaction: BackendTransaction<H>,
	/// The storage root after applying the transaction.
	pub transaction_storage_root: H::Out,
	/// Changes to the transaction index,
	#[cfg(feature = "std")]
	pub transaction_index_changes: Vec<IndexOperation>,
}

#[cfg(feature = "std")]
impl<H: Hasher> StorageChanges<H> {
	/// Deconstruct into the inner values
	pub fn into_inner(
		self,
	) -> (
		StorageCollection,
		ChildStorageCollection,
		OffchainChangesCollection,
		BackendTransaction<H>,
		H::Out,
		Vec<IndexOperation>,
	) {
		(
			self.main_storage_changes,
			self.child_storage_changes,
			self.offchain_storage_changes,
			self.transaction,
			self.transaction_storage_root,
			self.transaction_index_changes,
		)
	}
}

impl<H: Hasher> Default for StorageChanges<H> {
	fn default() -> Self {
		Self {
			main_storage_changes: Default::default(),
			child_storage_changes: Default::default(),
			offchain_storage_changes: Default::default(),
			transaction: BackendTransaction::with_hasher(Default::default()),
			transaction_storage_root: Default::default(),
			#[cfg(feature = "std")]
			transaction_index_changes: Default::default(),
		}
	}
}

/// Storage transactions are calculated as part of the `storage_root`.
/// These transactions can be reused for importing the block into the
/// storage. So, we cache them to not require a recomputation of those transactions.
struct StorageTransactionCache<H: Hasher> {
	/// Contains the changes for the main and the child storages as one transaction.
	transaction: BackendTransaction<H>,
	/// The storage root after applying the transaction.
	transaction_storage_root: H::Out,
}

impl<H: Hasher> StorageTransactionCache<H> {
	fn into_inner(self) -> (BackendTransaction<H>, H::Out) {
		(self.transaction, self.transaction_storage_root)
	}
}

impl<H: Hasher> Clone for StorageTransactionCache<H> {
	fn clone(&self) -> Self {
		Self {
			transaction: self.transaction.clone(),
			transaction_storage_root: self.transaction_storage_root,
		}
	}
}

impl<H: Hasher> core::fmt::Debug for StorageTransactionCache<H> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		let mut debug = f.debug_struct("StorageTransactionCache");

		#[cfg(feature = "std")]
		debug.field("transaction_storage_root", &self.transaction_storage_root);

		#[cfg(not(feature = "std"))]
		debug.field("transaction_storage_root", &self.transaction_storage_root.as_ref());

		debug.finish()
	}
}

impl<H: Hasher> OverlayedChanges<H> {
	/// Whether no changes are contained in the top nor in any of the child changes.
	pub fn is_empty(&self) -> bool {
		self.top.is_empty() && self.children.is_empty()
	}

	/// Ask to collect/not to collect extrinsics indices where key(s) has been changed.
	pub fn set_collect_extrinsics(&mut self, collect_extrinsics: bool) {
		self.collect_extrinsics = collect_extrinsics;
	}

	/// Returns a double-Option: None if the key is unknown (i.e. and the query should be referred
	/// to the backend); Some(None) if the key has been deleted. Some(Some(...)) for a key whose
	/// value has been set.
	pub fn storage(&mut self, key: &[u8]) -> Option<Option<&[u8]>> {
		self.top.get(key).map(|x| {
			let value = x.value();
			let size_read = value.map(|x| x.len() as u64).unwrap_or(0);
			self.stats.tally_read_modified(size_read);
			value.map(AsRef::as_ref)
		})
	}

	/// Should be called when there are changes that require to reset the
	/// `storage_transaction_cache`.
	fn mark_dirty(&mut self) {
		self.storage_transaction_cache = None;
	}

	/// Returns a double-Option: None if the key is unknown (i.e. and the query should be referred
	/// to the backend); Some(None) if the key has been deleted. Some(Some(...)) for a key whose
	/// value has been set.
	pub fn child_storage(&mut self, child_info: &ChildInfo, key: &[u8]) -> Option<Option<&[u8]>> {
		let map = self.children.get_mut(child_info.storage_key())?;
		let value = map.0.get(key)?.value();
		let size_read = value.map(|x| x.len() as u64).unwrap_or(0);
		self.stats.tally_read_modified(size_read);
		Some(value.map(AsRef::as_ref))
	}

	/// Set a new value for the specified key.
	///
	/// Can be rolled back or committed when called inside a transaction.
	pub fn set_storage(&mut self, key: StorageKey, val: Option<StorageValue>) {
		self.mark_dirty();

		let size_write = val.as_ref().map(|x| x.len() as u64).unwrap_or(0);
		self.stats.tally_write_overlay(size_write);
		let extrinsic_index = self.extrinsic_index();
		self.top.set(key, val, extrinsic_index);
	}

	/// Append a element to storage, init with existing value if first write.
	pub fn append_storage(
		&mut self,
		key: StorageKey,
		element: StorageValue,
		init: impl Fn() -> StorageValue,
	) {
		let extrinsic_index = self.extrinsic_index();
		let size_write = element.len() as u64;
		self.stats.tally_write_overlay(size_write);
		self.top.append_storage(key, element, init, extrinsic_index);
	}

	/// Set a new value for the specified key and child.
	///
	/// `None` can be used to delete a value specified by the given key.
	///
	/// Can be rolled back or committed when called inside a transaction.
	pub fn set_child_storage(
		&mut self,
		child_info: &ChildInfo,
		key: StorageKey,
		val: Option<StorageValue>,
	) {
		self.mark_dirty();

		let extrinsic_index = self.extrinsic_index();
		let size_write = val.as_ref().map(|x| x.len() as u64).unwrap_or(0);
		self.stats.tally_write_overlay(size_write);
		let storage_key = child_info.storage_key().to_vec();
		let top = &self.top;
		let (changeset, info) = self
			.children
			.entry(storage_key)
			.or_insert_with(|| (top.spawn_child(), child_info.clone()));
		let updatable = info.try_update(child_info);
		debug_assert!(updatable);
		changeset.set(key, val, extrinsic_index);
	}

	/// Clear child storage of given storage key.
	///
	/// Can be rolled back or committed when called inside a transaction.
	pub fn clear_child_storage(&mut self, child_info: &ChildInfo) -> u32 {
		self.mark_dirty();

		let extrinsic_index = self.extrinsic_index();
		let storage_key = child_info.storage_key().to_vec();
		let top = &self.top;
		let (changeset, info) = self
			.children
			.entry(storage_key)
			.or_insert_with(|| (top.spawn_child(), child_info.clone()));
		let updatable = info.try_update(child_info);
		debug_assert!(updatable);
		changeset.clear_where(|_, _| true, extrinsic_index)
	}

	/// Removes all key-value pairs which keys share the given prefix.
	///
	/// Can be rolled back or committed when called inside a transaction.
	pub fn clear_prefix(&mut self, prefix: &[u8]) -> u32 {
		self.mark_dirty();

		let extrinsic_index = self.extrinsic_index();
		self.top.clear_where(|key, _| key.starts_with(prefix), extrinsic_index)
	}

	/// Removes all key-value pairs which keys share the given prefix.
	///
	/// Can be rolled back or committed when called inside a transaction
	pub fn clear_child_prefix(&mut self, child_info: &ChildInfo, prefix: &[u8]) -> u32 {
		self.mark_dirty();

		let extrinsic_index = self.extrinsic_index();
		let storage_key = child_info.storage_key().to_vec();
		let top = &self.top;
		let (changeset, info) = self
			.children
			.entry(storage_key)
			.or_insert_with(|| (top.spawn_child(), child_info.clone()));
		let updatable = info.try_update(child_info);
		debug_assert!(updatable);
		changeset.clear_where(|key, _| key.starts_with(prefix), extrinsic_index)
	}

	/// Returns the current nesting depth of the transaction stack.
	///
	/// A value of zero means that no transaction is open and changes are committed on write.
	pub fn transaction_depth(&self) -> usize {
		// The top changeset and all child changesets transact in lockstep and are
		// therefore always at the same transaction depth.
		self.top.transaction_depth()
	}

	/// Start a new nested transaction.
	///
	/// This allows to either commit or roll back all changes that where made while this
	/// transaction was open. Any transaction must be closed by either `rollback_transaction` or
	/// `commit_transaction` before this overlay can be converted into storage changes.
	///
	/// Changes made without any open transaction are committed immediately.
	pub fn start_transaction(&mut self) {
		self.top.start_transaction();
		for (_, (changeset, _)) in self.children.iter_mut() {
			changeset.start_transaction();
		}
		self.offchain.overlay_mut().start_transaction();
	}

	/// Rollback the last transaction started by `start_transaction`.
	///
	/// Any changes made during that transaction are discarded. Returns an error if
	/// there is no open transaction that can be rolled back.
	pub fn rollback_transaction(&mut self) -> Result<(), NoOpenTransaction> {
		self.mark_dirty();

		self.top.rollback_transaction()?;
		retain_map(&mut self.children, |_, (changeset, _)| {
			changeset
				.rollback_transaction()
				.expect("Top and children changesets are started in lockstep; qed");
			!changeset.is_empty()
		});
		self.offchain
			.overlay_mut()
			.rollback_transaction_offchain()
			.expect("Top and offchain changesets are started in lockstep; qed");
		Ok(())
	}

	/// Commit the last transaction started by `start_transaction`.
	///
	/// Any changes made during that transaction are committed. Returns an error if there
	/// is no open transaction that can be committed.
	pub fn commit_transaction(&mut self) -> Result<(), NoOpenTransaction> {
		self.top.commit_transaction()?;
		for (_, (changeset, _)) in self.children.iter_mut() {
			changeset
				.commit_transaction()
				.expect("Top and children changesets are started in lockstep; qed");
		}
		self.offchain
			.overlay_mut()
			.commit_transaction_offchain()
			.expect("Top and offchain changesets are started in lockstep; qed");
		Ok(())
	}

	/// Call this before transferring control to the runtime.
	///
	/// This protects all existing transactions from being removed by the runtime.
	/// Calling this while already inside the runtime will return an error.
	pub fn enter_runtime(&mut self) -> Result<(), AlreadyInRuntime> {
		self.top.enter_runtime()?;
		for (_, (changeset, _)) in self.children.iter_mut() {
			changeset
				.enter_runtime()
				.expect("Top and children changesets are entering runtime in lockstep; qed")
		}
		self.offchain
			.overlay_mut()
			.enter_runtime()
			.expect("Top and offchain changesets are started in lockstep; qed");
		Ok(())
	}

	/// Call this when control returns from the runtime.
	///
	/// This rollbacks all dangling transaction left open by the runtime.
	/// Calling this while outside the runtime will return an error.
	pub fn exit_runtime(&mut self) -> Result<(), NotInRuntime> {
		self.top.exit_runtime()?;
		for (_, (changeset, _)) in self.children.iter_mut() {
			changeset
				.exit_runtime()
				.expect("Top and children changesets are entering runtime in lockstep; qed");
		}
		self.offchain
			.overlay_mut()
			.exit_runtime_offchain()
			.expect("Top and offchain changesets are started in lockstep; qed");
		Ok(())
	}

	/// Consume all changes (top + children) and return them.
	///
	/// After calling this function no more changes are contained in this changeset.
	///
	/// Panics:
	/// Panics if `transaction_depth() > 0`
	pub fn offchain_drain_committed(
		&mut self,
	) -> impl Iterator<Item = ((StorageKey, StorageKey), OffchainOverlayedChange)> {
		self.offchain.drain()
	}

	/// Get an iterator over all child changes as seen by the current transaction.
	pub fn children(
		&self,
	) -> impl Iterator<Item = (impl Iterator<Item = (&StorageKey, &OverlayedValue)>, &ChildInfo)> {
		self.children.values().map(|v| (v.0.changes(), &v.1))
	}

	/// Get an iterator over all child changes as seen by the current transaction.
	pub fn children_mut(
		&mut self,
	) -> impl Iterator<Item = (impl Iterator<Item = (&StorageKey, &mut OverlayedValue)>, &ChildInfo)>
	{
		self.children.values_mut().map(|v| (v.0.changes_mut(), &v.1))
	}

	/// Get an iterator over all top changes as been by the current transaction.
	pub fn changes(&self) -> impl Iterator<Item = (&StorageKey, &OverlayedValue)> {
		self.top.changes()
	}

	/// Get an iterator over all top changes as been by the current transaction.
	pub fn changes_mut(&mut self) -> impl Iterator<Item = (&StorageKey, &mut OverlayedValue)> {
		self.top.changes_mut()
	}

	/// Get an optional iterator over all child changes stored under the supplied key.
	pub fn child_changes(
		&self,
		key: &[u8],
	) -> Option<(impl Iterator<Item = (&StorageKey, &OverlayedValue)>, &ChildInfo)> {
		self.children.get(key).map(|(overlay, info)| (overlay.changes(), info))
	}

	/// Get an optional iterator over all child changes stored under the supplied key.
	pub fn child_changes_mut(
		&mut self,
		key: &[u8],
	) -> Option<(impl Iterator<Item = (&StorageKey, &mut OverlayedValue)>, &ChildInfo)> {
		self.children
			.get_mut(key)
			.map(|(overlay, info)| (overlay.changes_mut(), &*info))
	}

	/// Get an list of all index operations.
	pub fn transaction_index_ops(&self) -> &[IndexOperation] {
		&self.transaction_index_ops
	}

	/// Drain all changes into a [`StorageChanges`] instance. Leave empty overlay in place.
	pub fn drain_storage_changes<B: Backend<H>>(
		&mut self,
		backend: &B,
		state_version: StateVersion,
	) -> Result<StorageChanges<H>, DefaultError>
	where
		H::Out: Ord + Encode + 'static,
	{
		let (transaction, transaction_storage_root) = match self.storage_transaction_cache.take() {
			Some(cache) => cache.into_inner(),
			// If the transaction does not exist, we generate it.
			None => {
				self.storage_root(backend, state_version);
				self.storage_transaction_cache
					.take()
					.expect("`storage_transaction_cache` was just initialized; qed")
					.into_inner()
			},
		};

		use core::mem::take;
		let main_storage_changes =
			take(&mut self.top).drain_committed().map(|(k, v)| (k, v.to_option()));
		let child_storage_changes =
			take(&mut self.children).into_iter().map(|(key, (val, info))| {
				(key, (val.drain_committed().map(|(k, v)| (k, v.to_option())), info))
			});
		let offchain_storage_changes = self.offchain_drain_committed().collect();

		#[cfg(feature = "std")]
		let transaction_index_changes = std::mem::take(&mut self.transaction_index_ops);

		Ok(StorageChanges {
			main_storage_changes: main_storage_changes.collect(),
			child_storage_changes: child_storage_changes
				.map(|(sk, it)| (sk, it.0.collect()))
				.collect(),
			offchain_storage_changes,
			transaction,
			transaction_storage_root,
			#[cfg(feature = "std")]
			transaction_index_changes,
		})
	}

	/// Inserts storage entry responsible for current extrinsic index.
	#[cfg(test)]
	pub(crate) fn set_extrinsic_index(&mut self, extrinsic_index: u32) {
		self.top.set(EXTRINSIC_INDEX.to_vec(), Some(extrinsic_index.encode()), None);
	}

	/// Returns current extrinsic index to use in changes trie construction.
	/// None is returned if it is not set or changes trie config is not set.
	/// Persistent value (from the backend) can be ignored because runtime must
	/// set this index before first and unset after last extrinsic is executed.
	/// Changes that are made outside of extrinsics, are marked with
	/// `NO_EXTRINSIC_INDEX` index.
	fn extrinsic_index(&mut self) -> Option<u32> {
		self.collect_extrinsics.then(|| {
			self.storage(EXTRINSIC_INDEX)
				.and_then(|idx| idx.and_then(|idx| Decode::decode(&mut &*idx).ok()))
				.unwrap_or(NO_EXTRINSIC_INDEX)
		})
	}

	/// Generate the storage root using `backend` and all changes
	/// as seen by the current transaction.
	///
	/// Returns the storage root and whether it was already cached.
	pub fn storage_root<B: Backend<H>>(
		&mut self,
		backend: &B,
		state_version: StateVersion,
	) -> (H::Out, bool)
	where
		H::Out: Ord + Encode,
	{
		if let Some(cache) = &self.storage_transaction_cache {
			return (cache.transaction_storage_root, true)
		}

		let delta = self.top.changes_mut().map(|(k, v)| (&k[..], v.value().map(|v| &v[..])));

		let child_delta = self
			.children
			.values_mut()
			.map(|v| (&v.1, v.0.changes_mut().map(|(k, v)| (&k[..], v.value().map(|v| &v[..])))));

		let (root, transaction) = backend.full_storage_root(delta, child_delta, state_version);

		self.storage_transaction_cache =
			Some(StorageTransactionCache { transaction, transaction_storage_root: root });

		(root, false)
	}

	/// Generate the child storage root using `backend` and all child changes
	/// as seen by the current transaction.
	///
	/// Returns the child storage root and whether it was already cached.
	pub fn child_storage_root<B: Backend<H>>(
		&mut self,
		child_info: &ChildInfo,
		backend: &B,
		state_version: StateVersion,
	) -> Result<(H::Out, bool), B::Error>
	where
		H::Out: Ord + Encode + Decode,
	{
		let storage_key = child_info.storage_key();
		let prefixed_storage_key = child_info.prefixed_storage_key();

		if self.storage_transaction_cache.is_some() {
			let root = self
				.storage(prefixed_storage_key.as_slice())
				.map(|v| Ok(v.map(|v| v.to_vec())))
				.or_else(|| backend.storage(prefixed_storage_key.as_slice()).map(Some).transpose())
				.transpose()?
				.flatten()
				.and_then(|k| Decode::decode(&mut &k[..]).ok())
				// V1 is equivalent to V0 on empty root.
				.unwrap_or_else(empty_child_trie_root::<LayoutV1<H>>);

			return Ok((root, true))
		}

		let root = if let Some((changes, info)) = self.child_changes_mut(storage_key) {
			let delta = changes.map(|(k, v)| (k.as_ref(), v.value().map(AsRef::as_ref)));
			Some(backend.child_storage_root(info, delta, state_version))
		} else {
			None
		};

		let root = if let Some((root, is_empty, _)) = root {
			// We store update in the overlay in order to be able to use
			// 'self.storage_transaction' cache. This is brittle as it rely on Ext only querying
			// the trie backend for storage root.
			// A better design would be to manage 'child_storage_transaction' in a
			// similar way as 'storage_transaction' but for each child trie.
			self.set_storage(prefixed_storage_key.into_inner(), (!is_empty).then(|| root.encode()));

			self.mark_dirty();

			root
		} else {
			// empty overlay
			let root = backend
				.storage(prefixed_storage_key.as_slice())?
				.and_then(|k| Decode::decode(&mut &k[..]).ok())
				// V1 is equivalent to V0 on empty root.
				.unwrap_or_else(empty_child_trie_root::<LayoutV1<H>>);

			root
		};

		Ok((root, false))
	}

	/// Returns an iterator over the keys (in lexicographic order) following `key` (excluding `key`)
	/// alongside its value.
	pub fn iter_after(&mut self, key: &[u8]) -> impl Iterator<Item = (&[u8], &mut OverlayedValue)> {
		self.top.changes_after(key)
	}

	/// Returns an iterator over the keys (in lexicographic order) following `key` (excluding `key`)
	/// alongside its value for the given `storage_key` child.
	pub fn child_iter_after(
		&mut self,
		storage_key: &[u8],
		key: &[u8],
	) -> impl Iterator<Item = (&[u8], &mut OverlayedValue)> {
		self.children
			.get_mut(storage_key)
			.map(|(overlay, _)| overlay.changes_after(key))
			.into_iter()
			.flatten()
	}

	/// Read only access ot offchain overlay.
	pub fn offchain(&self) -> &OffchainOverlayedChanges {
		&self.offchain
	}

	/// Write a key value pair to the offchain storage overlay.
	pub fn set_offchain_storage(&mut self, key: &[u8], value: Option<&[u8]>) {
		use sp_core::offchain::STORAGE_PREFIX;
		match value {
			Some(value) => self.offchain.set(STORAGE_PREFIX, key, value),
			None => self.offchain.remove(STORAGE_PREFIX, key),
		}
	}

	/// Add transaction index operation.
	pub fn add_transaction_index(&mut self, op: IndexOperation) {
		self.transaction_index_ops.push(op)
	}
}

#[cfg(not(substrate_runtime))]
impl<H: Hasher> From<sp_core::storage::Storage> for OverlayedChanges<H> {
	fn from(storage: sp_core::storage::Storage) -> Self {
		Self {
			top: storage.top.into(),
			children: storage
				.children_default
				.into_iter()
				.map(|(k, v)| (k, (v.data.into(), v.child_info)))
				.collect(),
			..Default::default()
		}
	}
}

#[cfg(feature = "std")]
fn retain_map<K, V, F>(map: &mut Map<K, V>, f: F)
where
	K: std::cmp::Eq + std::hash::Hash,
	F: FnMut(&K, &mut V) -> bool,
{
	map.retain(f);
}

#[cfg(not(feature = "std"))]
fn retain_map<K, V, F>(map: &mut Map<K, V>, mut f: F)
where
	K: Ord,
	F: FnMut(&K, &mut V) -> bool,
{
	let old = core::mem::replace(map, Map::default());
	for (k, mut v) in old.into_iter() {
		if f(&k, &mut v) {
			map.insert(k, v);
		}
	}
}

/// An overlayed extension is either a mutable reference
/// or an owned extension.
#[cfg(feature = "std")]
pub enum OverlayedExtension<'a> {
	MutRef(&'a mut Box<dyn Extension>),
	Owned(Box<dyn Extension>),
}

/// Overlayed extensions which are sourced from [`Extensions`].
///
/// The sourced extensions will be stored as mutable references,
/// while extensions that are registered while execution are stored
/// as owned references. After the execution of a runtime function, we
/// can safely drop this object while not having modified the original
/// list.
#[cfg(feature = "std")]
pub struct OverlayedExtensions<'a> {
	extensions: Map<TypeId, OverlayedExtension<'a>>,
}

#[cfg(feature = "std")]
impl<'a> OverlayedExtensions<'a> {
	/// Create a new instance of overlaid extensions from the given extensions.
	pub fn new(extensions: &'a mut Extensions) -> Self {
		Self {
			extensions: extensions
				.iter_mut()
				.map(|(k, v)| (*k, OverlayedExtension::MutRef(v)))
				.collect(),
		}
	}

	/// Return a mutable reference to the requested extension.
	pub fn get_mut(&mut self, ext_type_id: TypeId) -> Option<&mut dyn Any> {
		self.extensions.get_mut(&ext_type_id).map(|ext| match ext {
			OverlayedExtension::MutRef(ext) => ext.as_mut_any(),
			OverlayedExtension::Owned(ext) => ext.as_mut_any(),
		})
	}

	/// Register extension `extension` with the given `type_id`.
	pub fn register(
		&mut self,
		type_id: TypeId,
		extension: Box<dyn Extension>,
	) -> Result<(), sp_externalities::Error> {
		match self.extensions.entry(type_id) {
			MapEntry::Vacant(vacant) => {
				vacant.insert(OverlayedExtension::Owned(extension));
				Ok(())
			},
			MapEntry::Occupied(_) => Err(sp_externalities::Error::ExtensionAlreadyRegistered),
		}
	}

	/// Deregister extension with the given `type_id`.
	///
	/// Returns `true` when there was an extension registered for the given `type_id`.
	pub fn deregister(&mut self, type_id: TypeId) -> bool {
		self.extensions.remove(&type_id).is_some()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{ext::Ext, new_in_mem, InMemoryBackend};
	use array_bytes::bytes2hex;
	use sp_core::{traits::Externalities, Blake2Hasher};
	use std::collections::BTreeMap;

	fn assert_extrinsics(
		overlay: &mut OverlayedChangeSet,
		key: impl AsRef<[u8]>,
		expected: Vec<u32>,
	) {
		assert_eq!(
			overlay.get(key.as_ref()).unwrap().extrinsics().into_iter().collect::<Vec<_>>(),
			expected
		)
	}

	#[test]
	fn overlayed_storage_works() {
		let mut overlayed = OverlayedChanges::<Blake2Hasher>::default();

		let key = vec![42, 69, 169, 142];

		assert!(overlayed.storage(&key).is_none());

		overlayed.start_transaction();

		overlayed.set_storage(key.clone(), Some(vec![1, 2, 3]));
		assert_eq!(overlayed.storage(&key).unwrap(), Some(&[1, 2, 3][..]));

		overlayed.commit_transaction().unwrap();

		assert_eq!(overlayed.storage(&key).unwrap(), Some(&[1, 2, 3][..]));

		overlayed.start_transaction();

		overlayed.set_storage(key.clone(), Some(vec![]));
		assert_eq!(overlayed.storage(&key).unwrap(), Some(&[][..]));

		overlayed.set_storage(key.clone(), None);
		assert!(overlayed.storage(&key).unwrap().is_none());

		overlayed.rollback_transaction().unwrap();

		assert_eq!(overlayed.storage(&key).unwrap(), Some(&[1, 2, 3][..]));

		overlayed.set_storage(key.clone(), None);
		assert!(overlayed.storage(&key).unwrap().is_none());
	}

	#[test]
	fn offchain_overlayed_storage_transactions_works() {
		use sp_core::offchain::STORAGE_PREFIX;
		fn check_offchain_content(
			state: &OverlayedChanges<Blake2Hasher>,
			nb_commit: usize,
			expected: Vec<(Vec<u8>, Option<Vec<u8>>)>,
		) {
			let mut state = state.clone();
			for _ in 0..nb_commit {
				state.commit_transaction().unwrap();
			}
			let offchain_data: Vec<_> = state.offchain_drain_committed().collect();
			let expected: Vec<_> = expected
				.into_iter()
				.map(|(key, value)| {
					let change = match value {
						Some(value) => OffchainOverlayedChange::SetValue(value),
						None => OffchainOverlayedChange::Remove,
					};
					((STORAGE_PREFIX.to_vec(), key), change)
				})
				.collect();
			assert_eq!(offchain_data, expected);
		}

		let mut overlayed = OverlayedChanges::default();

		let key = vec![42, 69, 169, 142];

		check_offchain_content(&overlayed, 0, vec![]);

		overlayed.start_transaction();

		overlayed.set_offchain_storage(key.as_slice(), Some(&[1, 2, 3][..]));
		check_offchain_content(&overlayed, 1, vec![(key.clone(), Some(vec![1, 2, 3]))]);

		overlayed.commit_transaction().unwrap();

		check_offchain_content(&overlayed, 0, vec![(key.clone(), Some(vec![1, 2, 3]))]);

		overlayed.start_transaction();

		overlayed.set_offchain_storage(key.as_slice(), Some(&[][..]));
		check_offchain_content(&overlayed, 1, vec![(key.clone(), Some(vec![]))]);

		overlayed.set_offchain_storage(key.as_slice(), None);
		check_offchain_content(&overlayed, 1, vec![(key.clone(), None)]);

		overlayed.rollback_transaction().unwrap();

		check_offchain_content(&overlayed, 0, vec![(key.clone(), Some(vec![1, 2, 3]))]);

		overlayed.set_offchain_storage(key.as_slice(), None);
		check_offchain_content(&overlayed, 0, vec![(key.clone(), None)]);
	}

	#[test]
	fn overlayed_storage_root_works() {
		let state_version = StateVersion::default();
		let initial: BTreeMap<_, _> = vec![
			(b"doe".to_vec(), b"reindeer".to_vec()),
			(b"dog".to_vec(), b"puppyXXX".to_vec()),
			(b"dogglesworth".to_vec(), b"catXXX".to_vec()),
			(b"doug".to_vec(), b"notadog".to_vec()),
		]
		.into_iter()
		.collect();
		let backend = InMemoryBackend::<Blake2Hasher>::from((initial, state_version));
		let mut overlay = OverlayedChanges::default();

		overlay.start_transaction();
		overlay.set_storage(b"dog".to_vec(), Some(b"puppy".to_vec()));
		overlay.set_storage(b"dogglesworth".to_vec(), Some(b"catYYY".to_vec()));
		overlay.set_storage(b"doug".to_vec(), Some(vec![]));
		overlay.commit_transaction().unwrap();

		overlay.start_transaction();
		overlay.set_storage(b"dogglesworth".to_vec(), Some(b"cat".to_vec()));
		overlay.set_storage(b"doug".to_vec(), None);

		{
			let mut ext = Ext::new(&mut overlay, &backend, None);
			let root = "39245109cef3758c2eed2ccba8d9b370a917850af3824bc8348d505df2c298fa";

			assert_eq!(bytes2hex("", &ext.storage_root(state_version)), root);
			// Calling a second time should use it from the cache
			assert_eq!(bytes2hex("", &ext.storage_root(state_version)), root);
		}

		// Check that the storage root is recalculated
		overlay.set_storage(b"doug2".to_vec(), Some(b"yes".to_vec()));

		let mut ext = Ext::new(&mut overlay, &backend, None);
		let root = "5c0a4e35cb967de785e1cb8743e6f24b6ff6d45155317f2078f6eb3fc4ff3e3d";
		assert_eq!(bytes2hex("", &ext.storage_root(state_version)), root);
	}

	#[test]
	fn overlayed_child_storage_root_works() {
		let state_version = StateVersion::default();
		let child_info = ChildInfo::new_default(b"Child1");
		let child_info = &child_info;
		let backend = new_in_mem::<Blake2Hasher>();
		let mut overlay = OverlayedChanges::<Blake2Hasher>::default();
		overlay.start_transaction();
		overlay.set_child_storage(child_info, vec![20], Some(vec![20]));
		overlay.set_child_storage(child_info, vec![30], Some(vec![30]));
		overlay.set_child_storage(child_info, vec![40], Some(vec![40]));
		overlay.commit_transaction().unwrap();
		overlay.set_child_storage(child_info, vec![10], Some(vec![10]));
		overlay.set_child_storage(child_info, vec![30], None);

		{
			let mut ext = Ext::new(&mut overlay, &backend, None);
			let child_root = "c02965e1df4dc5baf6977390ce67dab1d7a9b27a87c1afe27b50d29cc990e0f5";
			let root = "eafb765909c3ed5afd92a0c564acf4620d0234b31702e8e8e9b48da72a748838";

			assert_eq!(
				bytes2hex("", &ext.child_storage_root(child_info, state_version)),
				child_root,
			);

			assert_eq!(bytes2hex("", &ext.storage_root(state_version)), root);

			// Calling a second time should use it from the cache
			assert_eq!(
				bytes2hex("", &ext.child_storage_root(child_info, state_version)),
				child_root,
			);
		}
	}

	#[test]
	fn extrinsic_changes_are_collected() {
		let mut overlay = OverlayedChanges::<Blake2Hasher>::default();
		overlay.set_collect_extrinsics(true);

		overlay.start_transaction();

		overlay.set_storage(vec![100], Some(vec![101]));

		overlay.set_extrinsic_index(0);
		overlay.set_storage(vec![1], Some(vec![2]));

		overlay.set_extrinsic_index(1);
		overlay.set_storage(vec![3], Some(vec![4]));

		overlay.set_extrinsic_index(2);
		overlay.set_storage(vec![1], Some(vec![6]));

		assert_extrinsics(&mut overlay.top, vec![1], vec![0, 2]);
		assert_extrinsics(&mut overlay.top, vec![3], vec![1]);
		assert_extrinsics(&mut overlay.top, vec![100], vec![NO_EXTRINSIC_INDEX]);

		overlay.start_transaction();

		overlay.set_extrinsic_index(3);
		overlay.set_storage(vec![3], Some(vec![7]));

		overlay.set_extrinsic_index(4);
		overlay.set_storage(vec![1], Some(vec![8]));

		assert_extrinsics(&mut overlay.top, vec![1], vec![0, 2, 4]);
		assert_extrinsics(&mut overlay.top, vec![3], vec![1, 3]);
		assert_extrinsics(&mut overlay.top, vec![100], vec![NO_EXTRINSIC_INDEX]);

		overlay.rollback_transaction().unwrap();

		assert_extrinsics(&mut overlay.top, vec![1], vec![0, 2]);
		assert_extrinsics(&mut overlay.top, vec![3], vec![1]);
		assert_extrinsics(&mut overlay.top, vec![100], vec![NO_EXTRINSIC_INDEX]);
	}

	#[test]
	fn next_storage_key_change_works() {
		let mut overlay = OverlayedChanges::<Blake2Hasher>::default();
		overlay.start_transaction();
		overlay.set_storage(vec![20], Some(vec![20]));
		overlay.set_storage(vec![30], Some(vec![30]));
		overlay.set_storage(vec![40], Some(vec![40]));
		overlay.commit_transaction().unwrap();
		overlay.set_storage(vec![10], Some(vec![10]));
		overlay.set_storage(vec![30], None);

		// next_prospective < next_committed
		let next_to_5 = overlay.iter_after(&[5]).next().unwrap();
		assert_eq!(next_to_5.0.to_vec(), vec![10]);
		assert_eq!(next_to_5.1.value(), Some(&vec![10]));

		// next_committed < next_prospective
		let next_to_10 = overlay.iter_after(&[10]).next().unwrap();
		assert_eq!(next_to_10.0.to_vec(), vec![20]);
		assert_eq!(next_to_10.1.value(), Some(&vec![20]));

		// next_committed == next_prospective
		let next_to_20 = overlay.iter_after(&[20]).next().unwrap();
		assert_eq!(next_to_20.0.to_vec(), vec![30]);
		assert_eq!(next_to_20.1.value(), None);

		// next_committed, no next_prospective
		let next_to_30 = overlay.iter_after(&[30]).next().unwrap();
		assert_eq!(next_to_30.0.to_vec(), vec![40]);
		assert_eq!(next_to_30.1.value(), Some(&vec![40]));

		overlay.set_storage(vec![50], Some(vec![50]));
		// next_prospective, no next_committed
		let next_to_40 = overlay.iter_after(&[40]).next().unwrap();
		assert_eq!(next_to_40.0.to_vec(), vec![50]);
		assert_eq!(next_to_40.1.value(), Some(&vec![50]));
	}

	#[test]
	fn next_child_storage_key_change_works() {
		let child_info = ChildInfo::new_default(b"Child1");
		let child_info = &child_info;
		let child = child_info.storage_key();
		let mut overlay = OverlayedChanges::<Blake2Hasher>::default();
		overlay.start_transaction();
		overlay.set_child_storage(child_info, vec![20], Some(vec![20]));
		overlay.set_child_storage(child_info, vec![30], Some(vec![30]));
		overlay.set_child_storage(child_info, vec![40], Some(vec![40]));
		overlay.commit_transaction().unwrap();
		overlay.set_child_storage(child_info, vec![10], Some(vec![10]));
		overlay.set_child_storage(child_info, vec![30], None);

		// next_prospective < next_committed
		let next_to_5 = overlay.child_iter_after(child, &[5]).next().unwrap();
		assert_eq!(next_to_5.0.to_vec(), vec![10]);
		assert_eq!(next_to_5.1.value(), Some(&vec![10]));

		// next_committed < next_prospective
		let next_to_10 = overlay.child_iter_after(child, &[10]).next().unwrap();
		assert_eq!(next_to_10.0.to_vec(), vec![20]);
		assert_eq!(next_to_10.1.value(), Some(&vec![20]));

		// next_committed == next_prospective
		let next_to_20 = overlay.child_iter_after(child, &[20]).next().unwrap();
		assert_eq!(next_to_20.0.to_vec(), vec![30]);
		assert_eq!(next_to_20.1.value(), None);

		// next_committed, no next_prospective
		let next_to_30 = overlay.child_iter_after(child, &[30]).next().unwrap();
		assert_eq!(next_to_30.0.to_vec(), vec![40]);
		assert_eq!(next_to_30.1.value(), Some(&vec![40]));

		overlay.set_child_storage(child_info, vec![50], Some(vec![50]));
		// next_prospective, no next_committed
		let next_to_40 = overlay.child_iter_after(child, &[40]).next().unwrap();
		assert_eq!(next_to_40.0.to_vec(), vec![50]);
		assert_eq!(next_to_40.1.value(), Some(&vec![50]));
	}
}
