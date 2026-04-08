run-app:
	screen -S finance-tracker cargo run --release

start-caddy:
	sudo caddy start --config Caddyfile --envfile .env

stop-caddy:
	sudo caddy stop

clean:
	cargo clean
