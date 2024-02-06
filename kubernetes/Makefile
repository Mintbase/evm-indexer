# TODO - make this dynamic on network
NETWORK = mainnet

delete:
	kontemplate delete values-$(NETWORK).yaml

apply:
	kontemplate apply values-$(NETWORK).yaml

hard-restart: delete apply

restart:
	kubectl rollout restart deployment evm-indexer-$(NETWORK)
