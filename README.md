# OpenLLM

OpenLLM is an extensible server to optimize interactions with Ollama, HuggingFace, llama.cpp (llama-server), and other OpenAI-compatible APIs. It is maintained and developed by contributors of Solace.

OpenLLM provides an inference optimization engine, a model router, and a model registry.

## Quick Start

1. **Start the inference engine:**
   ```bash
   openllm-server --port 8080
   ```

2. **Use the TypeScript client:**
   ```ts
   import { createOpenLLMClient } from "@use-solace/openllm";
   
   const client = createOpenLLMClient({ engine: 8080 });
   const result = await client.inference({
     model_id: "llama3.1:8b",
     prompt: "Hello, how are you?",
   });
   ```

## Table of Contents

- [Engine](#engine)
- [Environment Variables](#environment-variables)
- [Model Registry](#model-registry)
- [API](#api)
- [Direct Client Usage](#direct-client-usage)
- [Error Handling](#error-handling)



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
openllm-server --port 9242
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

### API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | Health check and status |
| GET | `/v1/models` | List all registered models |
| POST | `/v1/models/register` | Register a new model |
| POST | `/v1/models/load` | Load a model into memory |
| POST | `/v1/models/unload/:id` | Unload a model |
| POST | `/v1/inference` | Non-streaming inference |
| POST | `/v1/inference/stream` | Streaming inference (SSE) |

## Environment Variables

Configure backend connections via environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `OLLAMA_URL` | `http://localhost:11434` | Ollama API endpoint |
| `LLAMA_CPP_URL` | `http://localhost:8080` | llama.cpp server endpoint |
| `HUGGINGFACE_URL` | `https://api-inference.huggingface.co` | HuggingFace API endpoint |
| `OPENAI_URL` | `https://api.openai.com/v1` | OpenAI API endpoint |
| `HUGGINGFACE_TOKEN` | - | HuggingFace API token |
| `OPENAI_API_KEY` | - | OpenAI API key |

## Direct Client Usage

Use the TypeScript client directly for programmatic access:

```ts
import { createOpenLLMClient } from "@use-solace/openllm";

const client = createOpenLLMClient({ engine: 8080 });

// Health check
const health = await client.health();
console.log("Status:", health.status);

// List models
const { models } = await client.listModels();

// Register a model
await client.registerModel({
  id: "mistral",
  name: "Mistral 7B",
  inference: "ollama",
  context: 8192,
  capabilities: ["chat"],
});

// Load a model
await client.loadModel({ model_id: "mistral" });

// Run inference
const result = await client.inference({
  model_id: "mistral",
  prompt: "What is the meaning of life?",
  max_tokens: 512,
  temperature: 0.7,
});

// Stream inference
await client.inferenceStream(
  {
    model_id: "mistral",
    prompt: "Tell me a story",
    max_tokens: 1024,
  },
  {
    onToken: (token) => process.stdout.write(token.token),
    onComplete: (response) => console.log("\nDone:", response.tokens_generated),
    onError: (error) => console.error("Error:", error.message),
  },
);

// Unload a model
await client.unloadModel("mistral");
```

### Complete Example

```ts
import { createOpenLLMClient } from "@use-solace/openllm";

async function main() {
  const client = createOpenLLMClient({ engine: 8080 });

  try {
    // Register
    await client.registerModel({
      id: "mistral",
      name: "Mistral 7B",
      inference: "ollama",
      context: 8192,
      capabilities: ["chat"],
    });

    // Load
    await client.loadModel({ model_id: "mistral" });

    // Stream inference
    let fullResponse = "";
    await client.inferenceStream(
      { model_id: "mistral", prompt: "Hello!", max_tokens: 256 },
      {
        onToken: (token) => {
          process.stdout.write(token.token);
          fullResponse += token.token;
        },
        onComplete: (resp) => console.log("\nTokens:", resp.tokens_generated),
      },
    );

    // Cleanup
    await client.unloadModel("mistral");
  } catch (error) {
    console.error("Error:", error);
  }
}
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
  } else if (error instanceof ModelNotLoadedError) {
    console.error("Model not loaded:", error.message);
    await client.loadModel({ model_id: error.message.match(/'([^']+)'/)?.[1] ?? "" });
  } else if (error instanceof InferenceError) {
    console.error("Inference failed:", error.message);
  } else if (error instanceof OpenLLMError) {
    console.error("API error:", error.code, error.message);
  }
}
```

## Architecture

OpenLLM consists of three main components:

### 1. Inference Engine (Rust)
- Written in Rust for maximum performance
- Supports SSE streaming for real-time token output
- Multi-backend support (Ollama, llama.cpp, HuggingFace, OpenAI)
- Configurable port and logging levels

### 2. Model Registry (TypeScript)
- In-memory model catalog with filtering
- Optional database persistence (PostgreSQL, MongoDB)
- Model routing based on capabilities and latency
- Full TypeScript type safety

### 3. API Layer (Elysia)
- HTTP API with optional model router
- Easy integration with existing Elysia apps
- Customizable route prefixes
- Streaming and non-streaming endpoints

## Development

### Building the Engine

```bash
cd engine
cargo build --release
```

### Building the NPM Package

```bash
cd npm
npm install
npm run build
```

### Running Tests

```bash
cd engine
cargo test

cd npm
npm test
```

## License

MIT
```