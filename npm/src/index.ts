export type {
  InferenceBackend,
  ModelCapability,
  LatencyProfile,
  ModelRegistryEntry,
  RegistryEntryInput,
  DatabaseConfig,
  ModelRegistryConfig,
  HealthResponse,
  ModelListResponse,
  RegisterModelResponse,
  LoadModelRequest,
  LoadModelResponse,
  UnloadModelResponse,
  InferenceRequest,
  InferenceResponse,
  StreamToken,
  StreamCallback,
  StreamCompleteCallback,
  StreamErrorCallback,
  StreamOptions,
  OpenLLMConfig,
  FindModelOptions,
  APIConfig,
  OpenLLMError,
  ModelNotFoundError,
  ModelNotLoadedError,
  InferenceError,
  ModelRegistryInstance,
  ModelRegistryImpl,
} from "./types.js";

export {
  ModelRegistry,
} from "./registry.js";

export type { ModelRegistry as ModelRegistryType } from "./registry.js";

export {
  OpenLLMClient,
  createOpenLLMClient,
} from "./client.js";

export { openllm } from "./elysia.js";

export const openllmAPI = {
  start: (config: import("./types.js").APIConfig = {}) => {
    const port = config.engine ?? 4292;
    return {
      start: (apiPort: number) => {
        console.log(`Starting OpenLLM API on port ${apiPort}, connected to engine on port ${port}`);
        console.log("Note: This is a client library. To start the actual server, use the Elysia plugin.");
      },
    };
  },
};
