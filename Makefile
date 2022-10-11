# build-release=RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown --locked
# build-debug=RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown --locked --features="debug-print"

# # Create wasm.gz, then Run local development chain
# .PHONY: build-start-server
# build-start-server: build start-server

.PHONY: clippy
clippy:
	cargo clippy

.PHONY: schema
schema:
	cargo run --example schema

# runs multi-contract unit tests
.PHONY: multitest 
multitest: 
	cargo test -p int_tests

.PHONY: integration-test
integration-test: build _integration-test
_integration-test:
	@# this line below doesn't work, but the point is you need to use npm v16
	@#. ${HOME}/.nvm/nvm.sh && nvm use 16
	npm --prefix int_tests/tests/ install
	npx ts-node ./int_tests/tests/integration.ts

# # `make start-server` on a different terminal first. Also need to `chmod u+x integration.sh`
# .PHONY: integration-test-shell
# integration-test-shell: build
# 	sleep 6
# 	if int_tests/tests/integration.sh; then echo -n '\a'; else echo -n '\a'; sleep 0.125; echo -n '\a'; fi

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

# # Run local development chain with four funded accounts (named a, b, c, and d)
# .PHONY: start-server
# start-server: # CTRL+C to stop
# 	docker run -it --rm \
# 		-p 26657:26657 -p 26656:26656 -p 1317:1317 \
# 		-v $$(pwd):/root/code \
# 		--name secretdev enigmampc/secret-network-sw-dev:latest

# Ctrl-C to exit terminal, but does not stop the server
.PHONY: start-server
start-server:
	docker start -a localsecret || true 
	docker run -it \
		-p 26657:26657 -p 26656:26656 -p 1317:1317 -p 5000:5000 -p 9090:9090 -p 9091:9091 \
		-v $$(pwd):/root/code \
		--name localsecret ghcr.io/scrtlabs/localsecret:v1.4.0

.PHONY: stop-server
stop-server:
	docker stop localsecret

.PHONY: reset-server
reset-server:
	docker stop localsecret || true
	docker rm localsecret || true
	docker run -it -p 9091:9091 -p 26657:26657 -p 1317:1317 -p 5000:5000 --name localsecret ghcr.io/scrtlabs/localsecret

# server needs to be running on another terminal
.PHONY: speedup-server
speedup-server:
	@# ok to reduce further to eg: 200ms
	docker exec localsecret sed -E -i '/timeout_(propose|prevote|precommit|commit)/s/[0-9]+m?s/500ms/' .secretd/config/config.toml
	docker stop localsecret
	docker start -a localsecret

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