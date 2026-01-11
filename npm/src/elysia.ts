import type { Elysia } from "elysia";
import type {
  APIConfig,
  InferenceBackend,
  InferenceRequest,
  ModelRegistryEntry,
  ModelRegistryImpl,
  RegisterModelRequest,
} from "./types.js";
import { createOpenLLMClient } from "./client.js";

export function openllm(config: APIConfig = {}) {
  const plugin = (app: Elysia) => {
    const enginePort = config.engine ?? 8080;
    const prefix = config.prefix ?? "/openllm";
    const enableRouter = config.modelrouter ?? false;
    const registry = config.registry as ModelRegistryImpl | undefined;

    const client = createOpenLLMClient({ engine: enginePort });

    app.get(`${prefix}/health`, async () => {
      return await client.health();
    });

    app.get(`${prefix}/models`, async () => {
      const result = await client.listModels();
      return result.models;
    });

    app.post(`${prefix}/models/register`, async ({ body }) => {
      const req = body as RegisterModelRequest;
      return await client.registerModel(req);
    });

    app.post(`${prefix}/models/load`, async ({ body }) => {
      const req = body as { model_id: string };
      return await client.loadModel(req);
    });

    app.post(`${prefix}/models/unload/:modelId`, async ({ params }) => {
      const modelId = params.modelId as string;
      return await client.unloadModel(modelId);
    });

    app.post(`${prefix}/inference`, async ({ body }) => {
      const req = body as InferenceRequest;
      return await client.inference(req);
    });

    if (enableRouter && registry) {
      app.post(`${prefix}/router/chat`, async ({ body }) => {
        const request = body as {
          prompt: string;
          options?: {
            model?: string;
            latency?: string;
            inference?: string;
            minContext?: number;
            max_tokens?: number;
            temperature?: number;
          };
        };
        const options = request.options ?? {};

        let model: ModelRegistryEntry | undefined;
        if (options.model) {
          model = registry.get(options.model);
          if (!model) {
            throw new Error(`Model '${options.model}' not found in registry`);
          }
        } else {
          model = registry.findOne({
            capability: "chat",
            latency: options.latency as any,
            inference: options.inference as InferenceBackend,
            minContext: options.minContext,
          });

          if (!model) {
            throw new Error(
              "No suitable model found for the given constraints",
            );
          }
        }

        return await client.inference({
          model_id: model.id,
          prompt: request.prompt,
          max_tokens: options.max_tokens,
          temperature: options.temperature,
        });
      });
    }

    return app;
  };

  return plugin;
}
