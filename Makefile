IMAGE ?= llm-schema-wasm-renderers
PORTS := -p 8000:8000 -p 8001:8001 -p 8002:8002 -p 8003:8003

.PHONY: help build playground stop clean

help: ## Show this help
	@grep -hE '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
	  | awk 'BEGIN{FS=":.*?## "}{printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}'

build: ## Build the Docker image (all deps + every WASM compiled)
	docker build -t $(IMAGE) .

playground: ## Run all four playgrounds, each on its own port (8000-8003)
	docker run --rm -it --name $(IMAGE) $(PORTS) $(IMAGE)

stop: ## Stop the running playground container
	-docker rm -f $(IMAGE)

clean: stop ## Remove the Docker image
	-docker rmi $(IMAGE)
