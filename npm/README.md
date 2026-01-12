# @use-solace/openllm

OpenLLM model registry and API client with full TypeScript support for interacting with the OpenLLM inference engine.

## Installation

```bash
npm install @use-solace/openllm@latest      # npm
bun add @use-solace/openllm@latest          # bun (recommended)
pnpm add @use-solace/openllm@latest         # pnpm
deno add @use-solace/openllm@latest         # deno
```

## Usage

### Model Registry

Define your model registry:

```ts
import { ModelRegistry } from "@use-solace/openllm";

export const models = ModelRegistry({
  entries: {
    "llama3.1-70b": {
      inference: "ollama",
      id: "llama3.1:70b",
      context: 8192,
      quant: "Q4_K_M",
      capabilities: ["chat"],
      latency: "slow",
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
});

// Usage examples
models.list();                   // returns all models
models.get("llama3.1:70b");      // returns data for llama3.1:70b
models.find({ capability: "chat", latency: "fast" });
models.findOne({ capability: "chat" });
models.has("llama3.1:70b");      // check if model exists
models.add("new-model", {        // add a new model
  inference: "ollama",
  id: "new-model",
  context: 4096,
  capabilities: ["chat"],
});
models.remove("new-model");      // remove a model
```

### API Client

Directly interact with the OpenLLM engine:

```ts
import { createOpenLLMClient } from "@use-solace/openllm";

const client = createOpenLLMClient({ engine: 8080 });

// Health check
const health = await client.health();

// List models
const { models } = await client.listModels();

// Register a model
await client.registerModel({
  id: "llama3.1:8b",
  name: "llama3.1-8b",
  inference: "ollama",
  context: 8192,
  quant: "Q4_K_M",
  capabilities: ["chat"],
  latency: "fast",
});

// Load a model
await client.loadModel({ model_id: "llama3.1:8b" });

// Run inference
const result = await client.inference({
  model_id: "llama3.1:8b",
  prompt: "Hello, how are you?",
  max_tokens: 512,
  temperature: 0.7,
});

// Stream inference
await client.inferenceStream(
  {
    model_id: "llama3.1:8b",
    prompt: "Tell me a story",
    max_tokens: 1024,
  },
  {
    onToken: (token) => console.log(token.token),
    onComplete: (response) => console.log("Done:", response),
    onError: (error) => console.error("Error:", error),
  },
);

// Unload a model
await client.unloadModel("llama3.1:8b");
```

### Elysia Plugin

```ts
import { Elysia } from "elysia";
import { openllm } from "@use-solace/openllm/elysia";
import { models } from "./registry.ts";

const app = new Elysia().use(
  openllm({
    prefix: "ollm",         // routes will be under /ollm/* instead of /openllm/*
    modelrouter: true,       // enable model router
    registry: models,        // pass the registry instance
    engine: 4292,            // openllm-server port
  }),
);

app.listen(3000);

console.log("Server running on localhost:3000");
```

Available endpoints:
- `GET /openllm/health` - Health check
- `GET /openllm/models` - List registered models
- `POST /openllm/models/register` - Register a new model
- `POST /openllm/models/load` - Load a model
- `POST /openllm/models/unload/:modelId` - Unload a model
- `POST /openllm/inference` - Run inference
- `POST /openllm/router/chat` - Chat with automatic model routing (if `modelrouter: true`)

## Types

The package provides full TypeScript type safety for all API interactions:

```ts
import type {
  InferenceBackend,
  ModelCapability,
  LatencyProfile,
  ModelRegistryEntry,
  InferenceRequest,
  InferenceResponse,
  StreamToken,
  // ... and more
} from "@use-solace/openllm";
```

## Error Handling

The package provides typed error classes:

```ts
import {
  OpenLLMError,
  ModelNotFoundError,
  ModelNotLoadedError,
  InferenceError,
} from "@use-solace/openllm";

try {
  await client.inference({ model_id: "unknown", prompt: "test" });
} catch (error) {
  if (error instanceof ModelNotFoundError) {
    console.error("Model not found:", error.message);
    console.error("Status code:", error.statusCode);
  }
}
```

## License

MIT
