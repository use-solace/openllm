# OpenLLM

OpenLLM is an extensible server to optimize interactions with Ollama, HuggingFace, llama.cpp (llama-server), and other OpenAI-compatible APIs. It is maintained and developed by the contributors of Solace.

OpenLLM provides an inference optimization engine, a model router, and a model registry.



## Engine

OpenLLM's inference engine (`openllm-server`) is a Rust-based application that handles inference efficiently and streams tokens using HTTP SSE endpoints.

Install via [cargo](https://crates.io/):

```bash
cargo install openllm-server
```

Run using

```bash
openllm-server
```

with custom port:

```bash
openllm-server 9242
```

## Model Registry

The Model Registry is provided by the [@use-solace/openllm](https://npmjs.com/package/@use-solace/openllm) package.

Install via npm (or your preferred JS package manager):

```bash
npm install @use-solace/openllm@latest      # npm
bun add @use-solace/openllm@latest          # bun (recommended)
pnpm add @use-solace/openllm@latest         # pnpm
deno add @use-solace/openllm@latest         # deno
```

Define your model registry:

```ts
import { ModelRegistry } from "@use-solace/openllm";

export const models = ModelRegistry(
  {
    "llama3.1-70b": {
      inference: "ollama",
      id: "llama3.1:70b",
      context: 8192,
      quant: "Q4_K_M",
      capabilities: ["chat"],      // chat, vision, embedding, completion
      latency: "slow",             // slow, fast, extreme
    },

    "llama3.1-8b": {
      inference: "llama",
      id: "llama3.1:8b",
      context: 8192,
      quant: "Q4_K_M",
      capabilities: ["chat", "completion"],
      latency: "fast",
    },
  },
  // Optional database settings
  {
    db: "postgres://postgres@localhost:5432/test",
    driver: "postgres",  // postgres, mongodb (more coming soon)
  }
);

// Usage examples
models.list();                   // returns all models
models.get("llama3.1:70b");      // returns data for llama3.1:70b
models.find({ capability: "chat", latency: "fast" });
```

**Notes:**

* `inference` refers to which backend the model uses (`llama` or `ollama`).
* `capabilities` defines supported tasks.
* `latency` is a routing hint.
* Optional database config allows persistent storage of the registry.



## API

OpenLLM provides an API via [Elysia](https://elysiajs.com/) using the [@use-solace/openllm](https://npmjs.com/package/@use-solace/openllm) package.

```ts
import { openllm } from "@use-solace/openllm";
import { models } from "./registry.ts";

const api = openllm.start({
    modelrouter: true, // enable model router
    registry: models,  // or a database connection string
    engine: 4292       // openllm-server port
});

api.start(2921); // runs API server on localhost:2921
```

### API as an Elysia Plugin

```ts
import { Elysia } from "elysia";
import { openllm } from "@use-solace/openllm/elysia";
import { models } from "./registry.ts";

const app = new Elysia().use(openllm({
    prefix: "ollm",     // routes will be under /ollm/* instead of /openllm/*
    modelrouter: true,  // enable model router
    registry: models,   // or database connection string
    engine: 4292        // openllm-server port
}));

app.listen(3000);

console.log("Server running on localhost:3000");
```