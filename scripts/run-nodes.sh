#!/bin/bash


killall sidevm-host;
killall screen;

# initialize
screen -S "sidevm-node-0" -d -m bash -c "ROCKET_PORT=8000 RUST_LOG='sidevm=info' WASMER_COMPILER=cranelift ./sidevm-host sideprog.wasm";
screen -S "sidevm-node-1" -d -m bash -c "ROCKET_PORT=8001 RUST_LOG='sidevm=info' WASMER_COMPILER=cranelift ./sidevm-host sideprog.wasm";
screen -S "sidevm-node-2" -d -m bash -c "ROCKET_PORT=8002 RUST_LOG='sidevm=info' WASMER_COMPILER=cranelift ./sidevm-host sideprog.wasm";
screen -S "sidevm-node-3" -d -m bash -c "ROCKET_PORT=8003 RUST_LOG='sidevm=info' WASMER_COMPILER=cranelift ./sidevm-host sideprog.wasm";

sleep 5;

curl -d "0" 127.0.0.1:8000/push/message
curl -d "1" 127.0.0.1:8001/push/message
curl -d "2" 127.0.0.1:8002/push/message
curl -d "3" 127.0.0.1:8003/push/message

sleep 1;

curl -d '{"command":"add_peer","data":"00000001"}' 127.0.0.1:8000/push/message
curl -d '{"command":"add_peer","data":"00000002"}' 127.0.0.1:8001/push/message
curl -d '{"command":"add_peer","data":"00000000"}' 127.0.0.1:8002/push/message
curl -d '{"command":"add_peer","data":"00000000"}' 127.0.0.1:8003/push/message

sleep 1;

curl -d '{"command":"start_optimize","data":""}' 127.0.0.1:8000/push/message
curl -d '{"command":"start_optimize","data":""}' 127.0.0.1:8001/push/message
curl -d '{"command":"start_optimize","data":""}' 127.0.0.1:8002/push/message
curl -d '{"command":"start_optimize","data":""}' 127.0.0.1:8003/push/message
