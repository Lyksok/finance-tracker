run-app-raspberry:
	screen -S finance-tracker ./finance_tracker

start-caddy:
	sudo caddy start --config Caddyfile --envfile .env

stop-caddy:
	sudo caddy stop

clean:
	cargo clean
