export type InferenceBackend = "ollama" | "llama" | "huggingface" | "openai";

export type ModelCapability = "chat" | "vision" | "embedding" | "completion";

export type LatencyProfile = "extreme" | "fast" | "slow";

export interface ModelRegistryEntry {
  id: string;
  name: string;
  inference: InferenceBackend;
  context: number;
  quant?: string;
  capabilities: ModelCapability[];
  latency?: LatencyProfile;
  size_bytes: number;
  loaded: boolean;
  loaded_at?: string;
}

export interface RegistryEntryInput {
  id: string;
  inference: InferenceBackend;
  context: number;
  quant?: string;
  capabilities: ModelCapability[];
  latency?: LatencyProfile;
}

export interface DatabaseConfig {
  db?: string;
  driver?: "postgres" | "mongodb";
}

export interface ModelRegistryConfig extends DatabaseConfig {
  entries: Record<string, RegistryEntryInput>;
}

export interface HealthResponse {
  status: string;
  timestamp: string;
  models_loaded: number;
}

export interface ModelListResponse {
  models: ModelRegistryEntry[];
}

export interface RegisterModelRequest {
  id: string;
  name: string;
  inference: InferenceBackend;
  context: number;
  quant?: string;
  capabilities: ModelCapability[];
  latency?: LatencyProfile;
  size_bytes?: number;
}

export interface RegisterModelResponse {
  success: boolean;
  model: ModelRegistryEntry;
  message: string;
}

export interface LoadModelRequest {
  model_id: string;
}

export interface LoadModelResponse {
  success: boolean;
  model_id: string;
  message: string;
}

export interface UnloadModelResponse {
  success: boolean;
  model_id: string;
  message: string;
}

export interface InferenceRequest {
  model_id: string;
  prompt: string;
  max_tokens?: number;
  temperature?: number;
}

export interface InferenceResponse {
  model_id: string;
  text: string;
  tokens_generated: number;
  finish_reason: string;
}

export interface StreamToken {
  token: string;
  token_id: number;
  complete: boolean;
}

export type StreamCallback = (token: StreamToken) => void;
export type StreamCompleteCallback = (response: InferenceResponse) => void;
export type StreamErrorCallback = (error: Error) => void;

export interface StreamOptions {
  onToken: StreamCallback;
  onComplete?: StreamCompleteCallback;
  onError?: StreamErrorCallback;
}

export interface OpenLLMConfig {
  engine?: string | number;
  timeout?: number;
}

export interface FindModelOptions {
  capability?: ModelCapability;
  latency?: LatencyProfile;
  inference?: InferenceBackend;
  minContext?: number;
  loaded?: boolean;
}

export interface APIConfig {
  modelrouter?: boolean;
  registry?: unknown | string;
  engine?: string | number;
  prefix?: string;
}

export class OpenLLMError extends Error {
  constructor(
    message: string,
    public code?: string,
    public statusCode?: number,
  ) {
    super(message);
    this.name = "OpenLLMError";
  }
}

export class ModelNotFoundError extends OpenLLMError {
  constructor(modelId: string) {
    super(`Model '${modelId}' not found`, "MODEL_NOT_FOUND", 404);
    this.name = "ModelNotFoundError";
  }
}

export class ModelNotLoadedError extends OpenLLMError {
  constructor(modelId: string) {
    super(`Model '${modelId}' is not loaded`, "MODEL_NOT_LOADED", 412);
    this.name = "ModelNotLoadedError";
  }
}

export class InferenceError extends OpenLLMError {
  constructor(message: string) {
    super(message, "INFERENCE_ERROR", 502);
    this.name = "InferenceError";
  }
}

export interface ModelRegistryInstance {
  list(): ReturnType<typeof import("./registry").ModelRegistryImpl.prototype.list>;
  get(id: string): ReturnType<typeof import("./registry").ModelRegistryImpl.prototype.get>;
  findOne(options: FindModelOptions): ReturnType<typeof import("./registry").ModelRegistryImpl.prototype.findOne>;
}

export type ModelRegistryImpl = ModelRegistryInstance;
