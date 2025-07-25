// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//! Primitives for tracking collations-related data.

use std::collections::{HashSet, VecDeque};

use futures::{future::BoxFuture, stream::FuturesUnordered};

use polkadot_node_network_protocol::{
	request_response::{incoming::OutgoingResponse, v2 as protocol_v2, IncomingRequest},
	PeerId,
};
use polkadot_node_primitives::PoV;
use polkadot_primitives::{
	vstaging::CandidateReceiptV2 as CandidateReceipt, CandidateHash, Hash, HeadData, Id as ParaId,
};

/// The status of a collation as seen from the collator.
#[derive(Clone, Debug, PartialEq)]
pub enum CollationStatus {
	/// The collation was created, but we did not advertise it to any validator.
	Created,
	/// The collation was advertised to at least one validator.
	Advertised,
	/// The collation was requested by at least one validator.
	Requested,
}

impl CollationStatus {
	/// Advance to the [`Self::Advertised`] status.
	///
	/// This ensures that `self` isn't already [`Self::Requested`].
	pub fn advance_to_advertised(&mut self) {
		if !matches!(self, Self::Requested) {
			*self = Self::Advertised;
		}
	}

	/// Advance to the [`Self::Requested`] status.
	pub fn advance_to_requested(&mut self) {
		*self = Self::Requested;
	}

	/// Return label for metrics.
	pub fn label(&self) -> &'static str {
		match self {
			CollationStatus::Created => "created",
			CollationStatus::Advertised => "advertised",
			CollationStatus::Requested => "requested",
		}
	}
}

/// A collation built by the collator.
pub struct Collation {
	/// Candidate receipt.
	pub receipt: CandidateReceipt,
	/// Proof to verify the state transition of the parachain.
	pub pov: PoV,
	/// Parent head-data
	pub parent_head_data: HeadData,
	/// Collation status.
	pub status: CollationStatus,
}

/// Stores the state for waiting collation fetches per relay parent.
#[derive(Default)]
pub struct WaitingCollationFetches {
	/// A flag indicating that we have an ongoing request.
	/// This limits the number of collations being sent at any moment
	/// of time to 1 for each relay parent.
	///
	/// If set to `true`, any new request will be queued.
	pub collation_fetch_active: bool,
	/// The collation fetches waiting to be fulfilled.
	pub req_queue: VecDeque<VersionedCollationRequest>,
	/// All peers that are waiting or actively uploading.
	///
	/// We will not accept multiple requests from the same peer, otherwise our DoS protection of
	/// moving on to the next peer after `MAX_UNSHARED_UPLOAD_TIME` would be pointless.
	pub waiting_peers: HashSet<(PeerId, CandidateHash)>,
}

/// Backwards-compatible wrapper for incoming collations requests.
pub enum VersionedCollationRequest {
	V2(IncomingRequest<protocol_v2::CollationFetchingRequest>),
}

impl From<IncomingRequest<protocol_v2::CollationFetchingRequest>> for VersionedCollationRequest {
	fn from(req: IncomingRequest<protocol_v2::CollationFetchingRequest>) -> Self {
		Self::V2(req)
	}
}

impl VersionedCollationRequest {
	/// Returns parachain id from the request payload.
	pub fn para_id(&self) -> ParaId {
		match self {
			VersionedCollationRequest::V2(req) => req.payload.para_id,
		}
	}

	/// Returns candidate hash from the request payload.
	pub fn candidate_hash(&self) -> CandidateHash {
		match self {
			VersionedCollationRequest::V2(req) => req.payload.candidate_hash,
		}
	}

	/// Returns relay parent from the request payload.
	pub fn relay_parent(&self) -> Hash {
		match self {
			VersionedCollationRequest::V2(req) => req.payload.relay_parent,
		}
	}

	/// Returns id of the peer the request was received from.
	pub fn peer_id(&self) -> PeerId {
		match self {
			VersionedCollationRequest::V2(req) => req.peer,
		}
	}

	/// Sends the response back to requester.
	pub fn send_outgoing_response(
		self,
		response: OutgoingResponse<protocol_v2::CollationFetchingResponse>,
	) -> Result<(), ()> {
		match self {
			VersionedCollationRequest::V2(req) => req.send_outgoing_response(response),
		}
	}
}

/// Result of the finished background send-collation task.
///
/// Note that if the timeout was hit the request doesn't get
/// aborted, it only indicates that we should start processing
/// the next one from the queue.
pub struct CollationSendResult {
	/// Candidate's relay parent.
	pub relay_parent: Hash,
	/// Candidate hash.
	pub candidate_hash: CandidateHash,
	/// Peer id.
	pub peer_id: PeerId,
	/// Whether the max unshared timeout was hit.
	pub timed_out: bool,
}

pub type ActiveCollationFetches = FuturesUnordered<BoxFuture<'static, CollationSendResult>>;
