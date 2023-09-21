- [Bridge-hub Parachains](#bridge-hub-parachains)
  - [Requirements for local run/testing](#requirements-for-local-runtesting)
  - [How to test local Rococo <-> Wococo bridge](#how-to-test-local-rococo---wococo-bridge)
    - [Run chains (Rococo + BridgeHub, Wococo + BridgeHub) with
      zombienet](#run-chains-rococo--bridgehub--assethub-wococo--bridgehub--assethub-with-zombienet)
    - [Run relayer (BridgeHubRococo, BridgeHubWococo)](#run-relayer-bridgehubrococo-bridgehubwococo)
      - [Run with script (alternative 1)](#run-with-script-alternative-1)
      - [Run with binary (alternative 2)](#run-with-binary-alternative-2)
    - [Send messages - transfer asset over bridge](#send-messages---transfer-asset-over-bridge)
  - [How to test live BridgeHubRococo/BridgeHubWococo](#how-to-test-live-bridgehubrococobridgehubwococo)
  - [How to test local BridgeHubKusama/BridgeHubPolkadot](#how-to-test-local-bridgehubkusamabridgehubpolkadot)
    - [1. Run chains (Kusama + BridgeHub + AssetHub, Polkadot + BridgeHub + AssetHub) with zombienet](#1-run-chains-kusama--bridgehub--assethub-polkadot--bridgehub--assethub-with-zombienet)
    - [2. Init bridge and run relayer (BridgeHubKusama, BridgeHubPolkadot)](#2-init-bridge-and-run-relayer-bridgehubkusama-bridgehubpolkadot)
    - [Send messages - transfer asset over bridge (DOTs/KSMs)](#4-send-messages---transfer-asset-over-bridge-dotsksms)

# Bridge-hub Parachains

_BridgeHub(s)_ are **_system parachains_** that will house trustless bridges from the local ecosystem to others. The
current trustless bridges planned for the BridgeHub(s) are:
- `BridgeHubPolkadot` system parachain:
	1. Polkadot <-> Kusama bridge
	2. Polkadot <-> Ethereum bridge (Snowbridge)
- `BridgeHubKusama` system parachain:
	1. Kusama <-> Polkadot bridge
	2. Kusama <-> Ethereum bridge The high-level
	responsibilities of each bridge living on BridgeHub:
- sync finality proofs between relay chains (or equivalent)
- sync finality proofs between BridgeHub parachains
- pass (XCM) messages between different BridgeHub parachains

![](./docs/bridge-hub-parachain-design.jpg "Basic deployment setup")

## Requirements for local run/testing

```
# Prepare empty directory for testing
mkdir -p ~/local_bridge_testing/bin
mkdir -p ~/local_bridge_testing/logs

---
# 1. Install zombienet
Go to: https://github.com/paritytech/zombienet/releases
Copy the apropriate binary (zombienet-linux) from the latest release to ~/local_bridge_testing/bin


---
# 2. Build polkadot binary

# If you want to test Kusama/Polkadot bridge, we need "sudo pallet + fast-runtime",
# so we need to use sudofi in polkadot directory.
#
# Install sudofi: (skip if already installed)
# cd <somewhere-outside-polkadot-sdk-git-repo-dir>
# git clone https://github.com/paritytech/parachain-utils.git
# cd parachain-utils # -> this is <parachain-utils-git-repo-dir>
# cargo build --release --bin sudofi
#
# cd <polkadot-sdk-git-repo-dir>/polkadot
# <parachain-utils-git-repo-dir>/target/release/sudofi

cd <polkadot-sdk-git-repo-dir>
cargo build --release --features fast-runtime --bin polkadot
cp target/release/polkadot ~/local_bridge_testing/bin/polkadot

cargo build --release --features fast-runtime --bin polkadot-prepare-worker
cp target/release/polkadot-prepare-worker ~/local_bridge_testing/bin/polkadot-prepare-worker

cargo build --release --features fast-runtime --bin polkadot-execute-worker
cp target/release/polkadot-execute-worker ~/local_bridge_testing/bin/polkadot-execute-worker


---
# 3. Build substrate-relay binary
git clone https://github.com/paritytech/parity-bridges-common.git
cd parity-bridges-common

# checkout desired branch or use master:
# git checkout -b master --track origin/master
# `polkadot-staging` (recommended) is stabilized and compatible for Cumulus releases
# `master` is latest development
git checkout -b polkadot-staging --track origin/polkadot-staging

cargo build --release -p substrate-relay
cp target/release/substrate-relay ~/local_bridge_testing/bin/substrate-relay


---
# 4. Build cumulus polkadot-parachain binary
cd <polkadot-sdk-git-repo-dir>

cargo build --release -p polkadot-parachain-bin
cp target/release/polkadot-parachain ~/local_bridge_testing/bin/polkadot-parachain
cp target/release/polkadot-parachain ~/local_bridge_testing/bin/polkadot-parachain-asset-hub
```


## How to test local Rococo <-> Wococo bridge

### Run chains (Rococo + BridgeHub + AssetHub, Wococo + BridgeHub + AssetHub) with zombienet

```
# Rococo + BridgeHubRococo + AssetHub for Rococo (mirroring Kusama)
POLKADOT_BINARY_PATH=~/local_bridge_testing/bin/polkadot \
POLKADOT_PARACHAIN_BINARY_PATH=~/local_bridge_testing/bin/polkadot-parachain \
POLKADOT_PARACHAIN_BINARY_PATH_FOR_ASSET_HUB_ROCOCO=~/local_bridge_testing/bin/polkadot-parachain-asset-hub \
	~/local_bridge_testing/bin/zombienet-linux --provider native spawn ./zombienet/bridge-hubs/bridge_hub_rococo_local_network.toml
```

```
# Wococo + BridgeHubWococo + AssetHub for Wococo (mirroring Polkadot)
POLKADOT_BINARY_PATH=~/local_bridge_testing/bin/polkadot \
POLKADOT_PARACHAIN_BINARY_PATH=~/local_bridge_testing/bin/polkadot-parachain \
POLKADOT_PARACHAIN_BINARY_PATH_FOR_ASSET_HUB_WOCOCO=~/local_bridge_testing/bin/polkadot-parachain-asset-hub \
	~/local_bridge_testing/bin/zombienet-linux --provider native spawn ./zombienet/bridge-hubs/bridge_hub_wococo_local_network.toml
```

### Run relayer (BridgeHubRococo, BridgeHubWococo)

**Accounts of BridgeHub parachains:**
- `Bob` is pallet owner of all bridge pallets

#### Run with script (alternative 1)
```
cd <cumulus-git-repo-dir>
./scripts/bridges_rococo_wococo.sh run-relay
```

#### Run with binary (alternative 2)
Need to wait for parachain activation (start producing blocks), then run:

```
# 1. Init bridges:

# Rococo -> Wococo
RUST_LOG=runtime=trace,rpc=trace,bridge=trace \
	~/local_bridge_testing/bin/substrate-relay init-bridge rococo-to-bridge-hub-wococo \
	--source-host localhost \
	--source-port 9942 \
	--source-version-mode Auto \
	--target-host localhost \
	--target-port 8945 \
	--target-version-mode Auto \
	--target-signer //Bob

# Wococo -> Rococo
RUST_LOG=runtime=trace,rpc=trace,bridge=trace \
	~/local_bridge_testing/bin/substrate-relay init-bridge wococo-to-bridge-hub-rococo \
	--source-host localhost \
	--source-port 9945 \
	--source-version-mode Auto \
	--target-host localhost \
	--target-port 8943 \
	--target-version-mode Auto \
	--target-signer //Bob

# 2. Relay relay-chain headers, parachain headers and messages**
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
```

**Check relay-chain headers relaying:**
- Rococo parachain: - https://polkadot.js.org/apps/?rpc=ws%3A%2F%2F127.0.0.1%3A8943#/chainstate - Pallet:
	**bridgeWococoGrandpa** - Keys: **bestFinalized()**
- Wococo parachain: - https://polkadot.js.org/apps/?rpc=ws%3A%2F%2F127.0.0.1%3A8945#/chainstate - Pallet:
	**bridgeRococoGrandpa** - Keys: **bestFinalized()**

**Check parachain headers relaying:**
- Rococo parachain: - https://polkadot.js.org/apps/?rpc=ws%3A%2F%2F127.0.0.1%3A8943#/chainstate - Pallet:
	**bridgeWococoParachain** - Keys: **bestParaHeads()**
- Wococo parachain: - https://polkadot.js.org/apps/?rpc=ws%3A%2F%2F127.0.0.1%3A8945#/chainstate - Pallet:
	**bridgeRococoParachain** - Keys: **bestParaHeads()**

### Send messages - transfer asset over bridge

TODO: see `# !!! READ HERE` above

## How to test live BridgeHubRococo/BridgeHubWococo
(here is still deployed older PoC from branch `origin/bko-transfer-asset-via-bridge`, which uses custom extrinsic, which
is going to be replaced by `pallet_xcm` usage)
- uses account seed on Live Rococo:Rockmine2
  ```
  cd <cumulus-git-repo-dir>
  ./scripts/bridges_rococo_wococo.sh transfer-asset-from-asset-hub-rococo
  ```

- open explorers: - Rockmine2 (see events `xcmpQueue.XcmpMessageSent`, `bridgeTransfer.ReserveAssetsDeposited`,
	`bridgeTransfer.TransferInitiated`)
	https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Fws-rococo-rockmine2-collator-node-0.parity-testnet.parity.io#/explorer
	- BridgeHubRococo (see `bridgeWococoMessages.MessageAccepted`)
	https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Frococo-bridge-hub-rpc.polkadot.io#/explorer - BridgeHubWococo (see
	`bridgeRococoMessages.MessagesReceived`)
	https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Fwococo-bridge-hub-rpc.polkadot.io#/explorer - Wockmint (see
	`xcmpQueue.Success` for `transfer-asset` and `xcmpQueue.Fail` for `ping-via-bridge`)
	https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Fwococo-wockmint-rpc.polkadot.io#/explorer - BridgeHubRococo (see
	`bridgeWococoMessages.MessagesDelivered`)

## How to test local BridgeHubKusama/BridgeHubPolkadot

Check [requirements](#requirements-for-local-runtesting) for "sudo pallet + fast-runtime".

### 1. Run chains (Kusama + BridgeHub + AssetHub, Polkadot + BridgeHub + AssetHub) with zombienet


```
cd <polkadot-sdk-git-repo-dir>

# Kusama + BridgeHubKusama + AssetHubKusama
POLKADOT_BINARY_PATH=~/local_bridge_testing/bin/polkadot \
POLKADOT_PARACHAIN_BINARY_PATH=~/local_bridge_testing/bin/polkadot-parachain \
POLKADOT_PARACHAIN_BINARY_PATH_FOR_ASSET_HUB_KUSAMA=~/local_bridge_testing/bin/polkadot-parachain-asset-hub \
	~/local_bridge_testing/bin/zombienet-linux --provider native spawn ./cumulus/zombienet/bridge-hubs/bridge_hub_kusama_local_network.toml
```

```
cd <polkadot-sdk-git-repo-dir>

# Polkadot + BridgeHubPolkadot + AssetHubPolkadot
POLKADOT_BINARY_PATH=~/local_bridge_testing/bin/polkadot \
POLKADOT_PARACHAIN_BINARY_PATH=~/local_bridge_testing/bin/polkadot-parachain \
POLKADOT_PARACHAIN_BINARY_PATH_FOR_ASSET_HUB_POLKADOT=~/local_bridge_testing/bin/polkadot-parachain-asset-hub \
	~/local_bridge_testing/bin/zombienet-linux --provider native spawn ./cumulus/zombienet/bridge-hubs/bridge_hub_polkadot_local_network.toml
```

### 2. Init bridge and run relayer (BridgeHubKusama, BridgeHubPolkadot)

```
cd <polkadot-sdk-git-repo-dir>

./cumulus/scripts/bridges_kusama_polkadot.sh run-relay
```

### 3. Initialize transfer asset over bridge (DOTs/KSMs)

This initialization does several things:
- creates `ForeignAssets` for wrappedDOTs/wrappedKSMs
- drips SA for AssetHubKusama on AssetHubPolkadot (and vice versa) which holds reserved assets on source chains
```
cd <polkadot-sdk-git-repo-dir>

./cumulus/scripts/bridges_kusama_polkadot.sh init-asset-hub-kusama-local
./cumulus/scripts/bridges_kusama_polkadot.sh init-bridge-hub-kusama-local
./cumulus/scripts/bridges_kusama_polkadot.sh init-asset-hub-polkadot-local
./cumulus/scripts/bridges_kusama_polkadot.sh init-bridge-hub-polkadot-local
```

### 4. Send messages - transfer asset over bridge (DOTs/KSMs)

Do (asset) transfers:
```
cd <polkadot-sdk-git-repo-dir>

# 2.5 KSMs from Kusama's Asset Hub to Polkadot's.
./cumulus/scripts/bridges_kusama_polkadot.sh reserve-transfer-assets-from-asset-hub-kusama-local
```
```
cd <polkadot-sdk-git-repo-dir>

# 30 DOTs from Polkadot's Asset Hub to Kusama's.
./cumulus/scripts/bridges_kusama_polkadot.sh reserve-transfer-assets-from-asset-hub-polkadot-local
```

- open explorers: (see zombienets)
	- AssetHubKusama (see events `xcmpQueue.XcmpMessageSent`, `polkadotXcm.Attempted`) https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:9910#/explorer
	- BridgeHubKusama (see `bridgePolkadotMessages.MessageAccepted`) https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:8943#/explorer
	- BridgeHubPolkadot (see `bridgeKusamaMessages.MessagesReceived`, `xcmpQueue.XcmpMessageSent`) https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:8945#/explorer
	- AssetHubPolkadot (see `foreignAssets.Issued`, `xcmpQueue.Success`) https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:9010#/explorer
	- BridgeHubKusama (see `bridgePolkadotMessages.MessagesDelivered`) https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:8943#/explorer