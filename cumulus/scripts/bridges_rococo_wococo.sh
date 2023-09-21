#!/bin/bash

# Expected sovereign accounts.
#
# Generated by:
#
#	#[test]
#	fn generate_sovereign_accounts() {
#		use sp_core::crypto::Ss58Codec;
#		use polkadot_parachain_primitives::primitives::Sibling;
#
#		parameter_types! {
#			pub UniversalLocationAHR: InteriorMultiLocation = X2(GlobalConsensus(Rococo), Parachain(1000));
#			pub UniversalLocationAHW: InteriorMultiLocation = X2(GlobalConsensus(Wococo), Parachain(1000));
#		}
#
#		// SS58=42
#		println!("GLOBAL_CONSENSUS_ROCOCO_SOVEREIGN_ACCOUNT=\"{}\"",
#				 frame_support::sp_runtime::AccountId32::new(
#					 GlobalConsensusConvertsFor::<UniversalLocationAHW, [u8; 32]>::convert_location(
#						 &MultiLocation { parents: 2, interior: X1(GlobalConsensus(Rococo)) }).unwrap()
#				 ).to_ss58check_with_version(42_u16.into())
#		);
#		println!("GLOBAL_CONSENSUS_ROCOCO_ASSET_HUB_ROCOCO_1000_SOVEREIGN_ACCOUNT=\"{}\"",
#				 frame_support::sp_runtime::AccountId32::new(
#					 GlobalConsensusParachainConvertsFor::<UniversalLocationAHW, [u8; 32]>::convert_location(
#						 &MultiLocation { parents: 2, interior: X2(GlobalConsensus(Rococo), Parachain(1000)) }).unwrap()
#				 ).to_ss58check_with_version(42_u16.into())
#		);
#		println!("ASSET_HUB_WOCOCO_SOVEREIGN_ACCOUNT_AT_BRIDGE_HUB_WOCOCO=\"{}\"",
#				 frame_support::sp_runtime::AccountId32::new(
#					 SiblingParachainConvertsVia::<Sibling, [u8; 32]>::convert_location(
#						 &MultiLocation { parents: 1, interior: X1(Parachain(1000)) }).unwrap()
#				 ).to_ss58check_with_version(42_u16.into())
#		);
#
#		// SS58=42
#		println!("GLOBAL_CONSENSUS_WOCOCO_SOVEREIGN_ACCOUNT=\"{}\"",
#				 frame_support::sp_runtime::AccountId32::new(
#					 GlobalConsensusConvertsFor::<UniversalLocationAHR, [u8; 32]>::convert_location(
#						 &MultiLocation { parents: 2, interior: X1(GlobalConsensus(Wococo)) }).unwrap()
#				 ).to_ss58check_with_version(42_u16.into())
#		);
#		println!("GLOBAL_CONSENSUS_WOCOCO_ASSET_HUB_WOCOCO_1000_SOVEREIGN_ACCOUNT=\"{}\"",
#				 frame_support::sp_runtime::AccountId32::new(
#					 GlobalConsensusParachainConvertsFor::<UniversalLocationAHR, [u8; 32]>::convert_location(
#						 &MultiLocation { parents: 2, interior: X2(GlobalConsensus(Wococo), Parachain(1000)) }).unwrap()
#				 ).to_ss58check_with_version(42_u16.into())
#		);
#		println!("ASSET_HUB_ROCOCO_SOVEREIGN_ACCOUNT_AT_BRIDGE_HUB_ROCOCO=\"{}\"",
#				 frame_support::sp_runtime::AccountId32::new(
#					 SiblingParachainConvertsVia::<Sibling, [u8; 32]>::convert_location(
#						 &MultiLocation { parents: 1, interior: X1(Parachain(1000)) }).unwrap()
#				 ).to_ss58check_with_version(42_u16.into())
#		);
#	}
GLOBAL_CONSENSUS_ROCOCO_SOVEREIGN_ACCOUNT="5GxRGwT8bU1JeBPTUXc7LEjZMxNrK8MyL2NJnkWFQJTQ4sii"
GLOBAL_CONSENSUS_ROCOCO_ASSET_HUB_ROCOCO_1000_SOVEREIGN_ACCOUNT="5CfNu7eH3SJvqqPt3aJh38T8dcFvhGzEohp9tsd41ANhXDnQ"
ASSET_HUB_WOCOCO_SOVEREIGN_ACCOUNT_AT_BRIDGE_HUB_WOCOCO="5Eg2fntNprdN3FgH4sfEaaZhYtddZQSQUqvYJ1f2mLtinVhV"
GLOBAL_CONSENSUS_WOCOCO_SOVEREIGN_ACCOUNT="5EWw2NzfPr2DCahourp33cya6bGWEJViTnJN6Z2ruFevpJML"
GLOBAL_CONSENSUS_WOCOCO_ASSET_HUB_WOCOCO_1000_SOVEREIGN_ACCOUNT="5EJX8L4dwGyYnCsjZ91LfWAsm3rCN8vY2AYvT4mauMEjsrQz"
ASSET_HUB_ROCOCO_SOVEREIGN_ACCOUNT_AT_BRIDGE_HUB_ROCOCO="5Eg2fntNprdN3FgH4sfEaaZhYtddZQSQUqvYJ1f2mLtinVhV"

function ensure_binaries() {
    if [[ ! -f ~/local_bridge_testing/bin/polkadot ]]; then
        echo "  Required polkadot binary '~/local_bridge_testing/bin/polkadot' does not exist!"
        echo "  You need to build it and copy to this location!"
        echo "  Please, check ./parachains/runtimes/bridge-hubs/README.md (Prepare/Build/Deploy)"
        exit 1
    fi
    if [[ ! -f ~/local_bridge_testing/bin/polkadot-parachain ]]; then
        echo "  Required polkadot-parachain binary '~/local_bridge_testing/bin/polkadot-parachain' does not exist!"
        echo "  You need to build it and copy to this location!"
        echo "  Please, check ./parachains/runtimes/bridge-hubs/README.md (Prepare/Build/Deploy)"
        exit 1
    fi
}

function ensure_relayer() {
    if [[ ! -f ~/local_bridge_testing/bin/substrate-relay ]]; then
        echo "  Required substrate-relay binary '~/local_bridge_testing/bin/substrate-relay' does not exist!"
        echo "  You need to build it and copy to this location!"
        echo "  Please, check ./parachains/runtimes/bridge-hubs/README.md (Prepare/Build/Deploy)"
        exit 1
    fi
}

function ensure_polkadot_js_api() {
    if ! which polkadot-js-api &> /dev/null; then
        echo ''
        echo 'Required command `polkadot-js-api` not in PATH, please, install, e.g.:'
        echo "npm install -g @polkadot/api-cli@beta"
        echo "      or"
        echo "yarn global add @polkadot/api-cli"
        echo ''
        exit 1
    fi
    if ! which jq &> /dev/null; then
        echo ''
        echo 'Required command `jq` not in PATH, please, install, e.g.:'
        echo "apt install -y jq"
        echo ''
        exit 1
    fi
    generate_hex_encoded_call_data "check" "--"
    local retVal=$?
    if [ $retVal -ne 0 ]; then
        echo ""
        echo ""
        echo "-------------------"
        echo "Installing (nodejs) sub module: $(dirname "$0")/generate_hex_encoded_call"
        pushd $(dirname "$0")/generate_hex_encoded_call
        npm install
        popd
    fi
}

function generate_hex_encoded_call_data() {
    local type=$1
    local endpoint=$2
    local output=$3
    shift
    shift
    shift
    echo "Input params: $@"

    node $(dirname "$0")/generate_hex_encoded_call "$type" "$endpoint" "$output" "$@"
    local retVal=$?

    if [ $type != "check" ]; then
        local hex_encoded_data=$(cat $output)
        echo "Generated hex-encoded bytes to file '$output': $hex_encoded_data"
    fi

    return $retVal
}

function transfer_balance() {
    local runtime_para_endpoint=$1
    local seed=$2
    local target_account=$3
    local amount=$4
    echo "  calling transfer_balance:"
    echo "      runtime_para_endpoint: ${runtime_para_endpoint}"
    echo "      seed: ${seed}"
    echo "      target_account: ${target_account}"
    echo "      amount: ${amount}"
    echo "--------------------------------------------------"

    polkadot-js-api \
        --ws "${runtime_para_endpoint}" \
        --seed "${seed?}" \
        tx.balances.transferAllowDeath \
            "${target_account}" \
            "${amount}"
}

function send_governance_transact() {
    local relay_url=$1
    local relay_chain_seed=$2
    local para_id=$3
    local hex_encoded_data=$4
    local require_weight_at_most_ref_time=$5
    local require_weight_at_most_proof_size=$6
    echo "  calling send_governance_transact:"
    echo "      relay_url: ${relay_url}"
    echo "      relay_chain_seed: ${relay_chain_seed}"
    echo "      para_id: ${para_id}"
    echo "      hex_encoded_data: ${hex_encoded_data}"
    echo "      require_weight_at_most_ref_time: ${require_weight_at_most_ref_time}"
    echo "      require_weight_at_most_proof_size: ${require_weight_at_most_proof_size}"
    echo "      params:"

    local dest=$(jq --null-input \
                    --arg para_id "$para_id" \
                    '{ "V3": { "parents": 0, "interior": { "X1": { "Parachain": $para_id } } } }')

    local message=$(jq --null-input \
                       --argjson hex_encoded_data $hex_encoded_data \
                       --arg require_weight_at_most_ref_time "$require_weight_at_most_ref_time" \
                       --arg require_weight_at_most_proof_size "$require_weight_at_most_proof_size" \
                       '
                       {
                          "V3": [
                                  {
                                    "UnpaidExecution": {
                                        "weight_limit": "Unlimited"
                                    }
                                  },
                                  {
                                    "Transact": {
                                      "origin_kind": "Superuser",
                                      "require_weight_at_most": {
                                        "ref_time": $require_weight_at_most_ref_time,
                                        "proof_size": $require_weight_at_most_proof_size,
                                      },
                                      "call": {
                                        "encoded": $hex_encoded_data
                                      }
                                    }
                                  }
                          ]
                        }
                        ')

    echo ""
    echo "          dest:"
    echo "${dest}"
    echo ""
    echo "          message:"
    echo "${message}"
    echo ""
    echo "--------------------------------------------------"

    polkadot-js-api \
        --ws "${relay_url?}" \
        --seed "${relay_chain_seed?}" \
        --sudo \
        tx.xcmPallet.send \
            "${dest}" \
            "${message}"
}

function open_hrmp_channels() {
    local relay_url=$1
    local relay_chain_seed=$2
    local sender_para_id=$3
    local recipient_para_id=$4
    local max_capacity=$5
    local max_message_size=$6
    echo "  calling open_hrmp_channels:"
    echo "      relay_url: ${relay_url}"
    echo "      relay_chain_seed: ${relay_chain_seed}"
    echo "      sender_para_id: ${sender_para_id}"
    echo "      recipient_para_id: ${recipient_para_id}"
    echo "      max_capacity: ${max_capacity}"
    echo "      max_message_size: ${max_message_size}"
    echo "      params:"
    echo "--------------------------------------------------"
    polkadot-js-api \
        --ws "${relay_url?}" \
        --seed "${relay_chain_seed?}" \
        --sudo \
        tx.hrmp.forceOpenHrmpChannel \
            ${sender_para_id} \
            ${recipient_para_id} \
            ${max_capacity} \
            ${max_message_size}
}

function set_storage() {
    local relay_url=$1
    local relay_chain_seed=$2
    local runtime_para_id=$3
    local runtime_para_endpoint=$4
    local items=$5
    echo "  calling set_storage:"
    echo "      relay_url: ${relay_url}"
    echo "      relay_chain_seed: ${relay_chain_seed}"
    echo "      runtime_para_id: ${runtime_para_id}"
    echo "      runtime_para_endpoint: ${runtime_para_endpoint}"
    echo "      items: ${items}"
    echo "      params:"

    # 1. generate data for Transact (System::set_storage)
    local tmp_output_file=$(mktemp)
    generate_hex_encoded_call_data "set-storage" "${runtime_para_endpoint}" "${tmp_output_file}" "$items"
    local hex_encoded_data=$(cat $tmp_output_file)

    # 2. trigger governance call
    send_governance_transact "${relay_url}" "${relay_chain_seed}" "${runtime_para_id}" "${hex_encoded_data}" 200000000 12000
}

function force_create_foreign_asset() {
    local relay_url=$1
    local relay_chain_seed=$2
    local runtime_para_id=$3
    local runtime_para_endpoint=$4
    local asset_multilocation=$5
    local asset_owner_account_id=$6
    local min_balance=$7
    local is_sufficient=$8
    echo "  calling force_create_foreign_asset:"
    echo "      relay_url: ${relay_url}"
    echo "      relay_chain_seed: ${relay_chain_seed}"
    echo "      runtime_para_id: ${runtime_para_id}"
    echo "      runtime_para_endpoint: ${runtime_para_endpoint}"
    echo "      asset_multilocation: ${asset_multilocation}"
    echo "      asset_owner_account_id: ${asset_owner_account_id}"
    echo "      min_balance: ${min_balance}"
    echo "      is_sufficient: ${is_sufficient}"
    echo "      params:"

    # 1. generate data for Transact (ForeignAssets::force_create)
    local tmp_output_file=$(mktemp)
    generate_hex_encoded_call_data "force-create-asset" "${runtime_para_endpoint}" "${tmp_output_file}" "$asset_multilocation" "$asset_owner_account_id" $is_sufficient $min_balance
    local hex_encoded_data=$(cat $tmp_output_file)

    # 2. trigger governance call
    send_governance_transact "${relay_url}" "${relay_chain_seed}" "${runtime_para_id}" "${hex_encoded_data}" 200000000 12000
}

function limited_reserve_transfer_assets() {
    local url=$1
    local seed=$2
    local destination=$3
    local beneficiary=$4
    local assets=$5
    local fee_asset_item=$6
    local weight_limit=$7
    echo "  calling limited_reserve_transfer_assets:"
    echo "      url: ${url}"
    echo "      seed: ${seed}"
    echo "      destination: ${destination}"
    echo "      beneficiary: ${beneficiary}"
    echo "      assets: ${assets}"
    echo "      fee_asset_item: ${fee_asset_item}"
    echo "      weight_limit: ${weight_limit}"
    echo ""
    echo "--------------------------------------------------"

    polkadot-js-api \
        --ws "${url?}" \
        --seed "${seed?}" \
        tx.polkadotXcm.limitedReserveTransferAssets \
            "${destination}" \
            "${beneficiary}" \
            "${assets}" \
            "${fee_asset_item}" \
            "${weight_limit}"
}

function init_ro_wo() {
    ensure_relayer

    RUST_LOG=runtime=trace,rpc=trace,bridge=trace \
        ~/local_bridge_testing/bin/substrate-relay init-bridge rococo-to-bridge-hub-wococo \
	--source-host localhost \
	--source-port 9942 \
	--source-version-mode Auto \
	--target-host localhost \
	--target-port 8945 \
	--target-version-mode Auto \
	--target-signer //Bob
}

function init_wo_ro() {
    ensure_relayer

    RUST_LOG=runtime=trace,rpc=trace,bridge=trace \
        ~/local_bridge_testing/bin/substrate-relay init-bridge wococo-to-bridge-hub-rococo \
        --source-host localhost \
        --source-port 9945 \
        --source-version-mode Auto \
        --target-host localhost \
        --target-port 8943 \
        --target-version-mode Auto \
        --target-signer //Bob
}

function run_relay() {
    ensure_relayer

    RUST_LOG=runtime=trace,rpc=trace,bridge=trace \
        ~/local_bridge_testing/bin/substrate-relay relay-headers-and-messages bridge-hub-rococo-bridge-hub-wococo \
        --rococo-host localhost \
        --rococo-port 9942 \
        --rococo-version-mode Auto \
        --bridge-hub-rococo-host localhost \
        --bridge-hub-rococo-port 8943 \
        --bridge-hub-rococo-version-mode Auto \
        --bridge-hub-rococo-signer //Charlie \
        --wococo-headers-to-bridge-hub-rococo-signer //Bob \
        --wococo-parachains-to-bridge-hub-rococo-signer //Bob \
        --bridge-hub-rococo-transactions-mortality 4 \
        --wococo-host localhost \
        --wococo-port 9945 \
        --wococo-version-mode Auto \
        --bridge-hub-wococo-host localhost \
        --bridge-hub-wococo-port 8945 \
        --bridge-hub-wococo-version-mode Auto \
        --bridge-hub-wococo-signer //Charlie \
        --rococo-headers-to-bridge-hub-wococo-signer //Bob \
        --rococo-parachains-to-bridge-hub-wococo-signer //Bob \
        --bridge-hub-wococo-transactions-mortality 4 \
        --lane 00000001
}

case "$1" in
  run-relay)
    init_ro_wo
    init_wo_ro
    run_relay
    ;;
  init-asset-hub-rococo-local)
      ensure_polkadot_js_api
      # create foreign assets for native Wococo token (governance call on Rococo)
      force_create_foreign_asset \
          "ws://127.0.0.1:9942" \
          "//Alice" \
          1000 \
          "ws://127.0.0.1:9910" \
          "$(jq --null-input '{ "parents": 2, "interior": { "X1": { "GlobalConsensus": "Wococo" } } }')" \
          "$GLOBAL_CONSENSUS_WOCOCO_SOVEREIGN_ACCOUNT" \
          10000000000 \
          true
      # drip SA which holds reserves
      transfer_balance \
          "ws://127.0.0.1:9910" \
          "//Alice" \
          "$GLOBAL_CONSENSUS_WOCOCO_ASSET_HUB_WOCOCO_1000_SOVEREIGN_ACCOUNT" \
          $((1000000000 + 50000000000 * 20))
      # HRMP
      open_hrmp_channels \
          "ws://127.0.0.1:9942" \
          "//Alice" \
          1000 1013 4 524288
      open_hrmp_channels \
          "ws://127.0.0.1:9942" \
          "//Alice" \
          1013 1000 4 524288
      ;;
  init-bridge-hub-rococo-local)
      ensure_polkadot_js_api
      # SA of sibling asset hub pays for the execution
      transfer_balance \
          "ws://127.0.0.1:8943" \
          "//Alice" \
          "$ASSET_HUB_ROCOCO_SOVEREIGN_ACCOUNT_AT_BRIDGE_HUB_ROCOCO" \
          $((1000000000 + 50000000000 * 20))
      ;;
  init-asset-hub-wococo-local)
      ensure_polkadot_js_api
      # set Wococo flavor - set_storage with:
      # - `key` is `HexDisplay::from(&asset_hub_rococo_runtime::xcm_config::Flavor::key())`
      # - `value` is `HexDisplay::from(&asset_hub_rococo_runtime::RuntimeFlavor::Wococo.encode())`
      set_storage \
          "ws://127.0.0.1:9945" \
          "//Alice" \
          1000 \
          "ws://127.0.0.1:9010" \
          "$(jq --null-input '[["0x48297505634037ef48c848c99c0b1f1b", "0x01"]]')"
      # create foreign assets for native Rococo token (governance call on Wococo)
      force_create_foreign_asset \
          "ws://127.0.0.1:9945" \
          "//Alice" \
          1000 \
          "ws://127.0.0.1:9010" \
          "$(jq --null-input '{ "parents": 2, "interior": { "X1": { "GlobalConsensus": "Rococo" } } }')" \
          "$GLOBAL_CONSENSUS_ROCOCO_SOVEREIGN_ACCOUNT" \
          10000000000 \
          true
      # drip SA which holds reserves
      transfer_balance \
          "ws://127.0.0.1:9010" \
          "//Alice" \
          "$GLOBAL_CONSENSUS_ROCOCO_ASSET_HUB_ROCOCO_1000_SOVEREIGN_ACCOUNT" \
          $((1000000000 + 50000000000 * 20))
      # HRMP
      open_hrmp_channels \
          "ws://127.0.0.1:9945" \
          "//Alice" \
          1000 1014 4 524288
      open_hrmp_channels \
          "ws://127.0.0.1:9945" \
          "//Alice" \
          1014 1000 4 524288
      ;;
  init-bridge-hub-wococo-local)
      # set Wococo flavor - set_storage with:
      # - `key` is `HexDisplay::from(&bridge_hub_rococo_runtime::xcm_config::Flavor::key())`
      # - `value` is `HexDisplay::from(&bridge_hub_rococo_runtime::RuntimeFlavor::Wococo.encode())`
      set_storage \
          "ws://127.0.0.1:9945" \
          "//Alice" \
          1014 \
          "ws://127.0.0.1:8945" \
          "$(jq --null-input '[["0x48297505634037ef48c848c99c0b1f1b", "0x01"]]')"
      # SA of sibling asset hub pays for the execution
      transfer_balance \
          "ws://127.0.0.1:8945" \
          "//Alice" \
          "$ASSET_HUB_WOCOCO_SOVEREIGN_ACCOUNT_AT_BRIDGE_HUB_WOCOCO" \
          $((1000000000 + 50000000000 * 20))
      ;;
  reserve-transfer-assets-from-asset-hub-rococo-local)
      ensure_polkadot_js_api
      # send ROCs to Alice account on AHW
      limited_reserve_transfer_assets \
          "ws://127.0.0.1:9910" \
          "//Alice" \
          "$(jq --null-input '{ "V3": { "parents": 2, "interior": { "X2": [ { "GlobalConsensus": "Wococo" }, { "Parachain": 1000 } ] } } }')" \
          "$(jq --null-input '{ "V3": { "parents": 0, "interior": { "X1": { "AccountId32": { "id": [212, 53, 147, 199, 21, 253, 211, 28, 97, 20, 26, 189, 4, 169, 159, 214, 130, 44, 133, 88, 133, 76, 205, 227, 154, 86, 132, 231, 165, 109, 162, 125] } } } } }')" \
          "$(jq --null-input '{ "V3": [ { "id": { "Concrete": { "parents": 1, "interior": "Here" } }, "fun": { "Fungible": 200000000000 } } ] }')" \
          0 \
          "Unlimited"
      ;;
  reserve-transfer-assets-from-asset-hub-wococo-local)
      ensure_polkadot_js_api
      # send WOCs to Alice account on AHR
      limited_reserve_transfer_assets \
          "ws://127.0.0.1:9010" \
          "//Alice" \
          "$(jq --null-input '{ "V3": { "parents": 2, "interior": { "X2": [ { "GlobalConsensus": "Rococo" }, { "Parachain": 1000 } ] } } }')" \
          "$(jq --null-input '{ "V3": { "parents": 0, "interior": { "X1": { "AccountId32": { "id": [212, 53, 147, 199, 21, 253, 211, 28, 97, 20, 26, 189, 4, 169, 159, 214, 130, 44, 133, 88, 133, 76, 205, 227, 154, 86, 132, 231, 165, 109, 162, 125] } } } } }')" \
          "$(jq --null-input '{ "V3": [ { "id": { "Concrete": { "parents": 1, "interior": "Here" } }, "fun": { "Fungible": 150000000000 } } ] }')" \
          0 \
          "Unlimited"
      ;;
  stop)
    pkill -f polkadot
    pkill -f parachain
    ;;
  import)
    # to avoid trigger anything here
    ;;
  *)
    echo "A command is require. Supported commands for:
    Local (zombienet) run:
          - run-relay
          - init-asset-hub-rococo-local
          - init-bridge-hub-rococo-local
          - init-asset-hub-wococo-local
          - init-bridge-hub-wococo-local
          - reserve-transfer-assets-from-asset-hub-rococo-local
          - reserve-transfer-assets-from-asset-hub-wococo-local";
    exit 1
    ;;
esac