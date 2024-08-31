docker-build:
	docker build . -t maelstrom-container

docker-run:
	cargo build --manifest-path=tea/Cargo.toml && \
		docker run --name maelstrom --rm -v ./tea/target/:/builds/ -v ./debug-logs:/store/ maelstrom-container \
		test -w echo --bin /builds/debug/echo --node-count 1 --time-limit 10

docker-stop: 
	docker stop maelstrom

docker-run-detached:
	docker run --name maelstrom --rm -d -v ./tea/target/:/builds/ maelstrom-container
