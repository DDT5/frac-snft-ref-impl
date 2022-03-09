# build-release=RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown --locked
# build-debug=RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown --locked --features="debug-print"

# Create wasm.gz, then Run local development chain
.PHONY: build-start-server
build-start-server: build start-server

.PHONY: build _build
build: _build compress-wasm
_build:
	RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown --features="debug-print"

.PHONY: compress-wasm
compress-wasm:
	@# compress factory contract
	cp ./target/wasm32-unknown-unknown/release/factory.wasm ./factory.wasm
	@## The following line is not necessary, may work only on linux (extra size optimization)
	wasm-opt -Os ./factory.wasm -o ./factory.wasm
	cat ./factory.wasm | gzip -9 > ./factory.wasm.gz
	rm ./factory.wasm
	
	@# compress ftoken contract
	cp ./target/wasm32-unknown-unknown/release/ftoken.wasm ./ftoken.wasm
	wasm-opt -Os ./ftoken.wasm -o ./ftoken.wasm
	cat ./ftoken.wasm | gzip -9 > ./ftoken.wasm.gz
	rm ./ftoken.wasm

# Run local development chain with four funded accounts (named a, b, c, and d)
.PHONY: start-server
start-server: # CTRL+C to stop
	docker run -it --rm \
		-p 26657:26657 -p 26656:26656 -p 1317:1317 \
		-v $$(pwd):/root/code \
		--name secretdev enigmampc/secret-network-sw-dev:latest

# like build-mainnet, but slower and more deterministic
.PHONY: build-mainnet-reproducible
build-mainnet-reproducible:
	docker run --rm -v "$$(pwd)":/contract \
		--mount type=volume,source="$$(basename "$$(pwd)")_cache",target=/code/target \
		--mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
		enigmampc/secret-contract-optimizer:1.0.6

.PHONY: clean
clean:
	cargo clean
	-rm -f ./*.wasm ./*.wasm.gz

.PHONY: sudo-clean
sudo-clean:
	sudo rm -rf target
	cargo clean
	-rm -f ./*.wasm ./*.wasm.gz