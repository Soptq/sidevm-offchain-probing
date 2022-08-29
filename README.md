# Sidevm Offchain Probing

An almost-done offchain probing system for Phala Network using SideVM.

Currently, all parameters are either hardcoded or set by the user using `push_message`. They will be dynamically assigned by fat contract using on-chain data once the SideVM SDK providers a way to read on-chain storages. 

## Test

You can setup a 4 nodes test environment by referring `./scripts/run-4-nodes.sh`.

After letting the cluster run and iterate for a while, you can fetch the status by getting `/status` endpoint, which contains information like precision or current epoch.