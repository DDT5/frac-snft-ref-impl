# build-release=RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown --locked
# build-debug=RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown --locked --features="debug-print"

# Create wasm.gz, then Run local development chain
.PHONY: build-start-server
build-start-server: build start-server

# runs multi-contract unit tests
.PHONY: multitest 
multitest: 
	cargo test -p int_tests

# `make start-server` on a different terminal first. Also need to `chmod u+x integration.sh`
.PHONY: integration-test
integration-test: build
	sleep 6
	if int_tests/tests/integration.sh; then echo -n '\a'; else echo -n '\a'; sleep 0.125; echo -n '\a'; fi

.PHONY: build _build
build: _build compress-wasm
_build:
	RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown --features="debug-print"

.PHONY: compress-wasm
compress-wasm:
	@# compress fractionalizer contract
	cp ./target/wasm32-unknown-unknown/release/fractionalizer.wasm ./fractionalizer.wasm
	@## The following line is not necessary, may work only on linux (extra size optimization)
	wasm-opt -Os ./fractionalizer.wasm -o ./fractionalizer.wasm
	cat ./fractionalizer.wasm | gzip -9 > ./fractionalizer.wasm.gz
	rm ./fractionalizer.wasm
	
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
		enigmampc/secret-contract-optimizer:1.0.9

.PHONY: doc
doc:
	rm -rf ./target/doc
	cargo doc --no-deps --workspace --exclude snip20-reference-impl \
	--exclude snip721-reference-impl --exclude int_tests
	rm -rf ../Doc/docs
	cp -r ./target/doc ../Doc/docs

.PHONY: clean
clean:
	cargo clean
	-rm -f ./*.wasm ./*.wasm.gz

.PHONY: sudo-clean
sudo-clean:
	sudo rm -rf target
	cargo clean
	-rm -f ./*.wasm ./*.wasm.gz