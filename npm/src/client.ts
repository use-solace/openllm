import type {
  HealthResponse,
  InferenceRequest,
  InferenceResponse,
  LoadModelRequest,
  LoadModelResponse,
  ModelListResponse,
  ModelNotFoundError,
  ModelNotLoadedError,
  OpenLLMConfig,
  OpenLLMError,
  RegisterModelRequest,
  RegisterModelResponse,
  StreamOptions,
  StreamToken,
  UnloadModelResponse,
} from "./types.js";

export class OpenLLMClient {
  private baseUrl: string;
  private timeout: number;

  constructor(config: OpenLLMConfig = {}) {
    this.baseUrl = `http://localhost:${config.engine ?? 8080}`;
    this.timeout = config.timeout ?? 30000;
  }

  private async request<T>(
    endpoint: string,
    options: RequestInit = {},
  ): Promise<T> {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.timeout);

    try {
      const response = await fetch(`${this.baseUrl}${endpoint}`, {
        ...options,
        signal: controller.signal,
        headers: {
          "Content-Type": "application/json",
          ...options.headers,
        },
      });

      clearTimeout(timeoutId);

      if (!response.ok) {
        const errorText = await response.text();
        throw this.createError(response.status, errorText);
      }

      return (await response.json()) as T;
    } catch (error) {
      clearTimeout(timeoutId);
      if (error instanceof Error) {
        if (error.name === "AbortError") {
          throw new Error(`Request timeout after ${this.timeout}ms`);
        }
        throw error;
      }
      throw error;
    }
  }

  private createError(status: number, message: string): OpenLLMError {
    if (status === 404) {
      const match = message.match(/Model '([^']+)'/);
      if (match) {
        const error = new Error(message) as ModelNotFoundError;
        error.name = "ModelNotFoundError";
        error.code = "MODEL_NOT_FOUND";
        error.statusCode = status;
        return error;
      }
    }
    if (status === 412) {
      const match = message.match(/Model '([^']+)'/);
      if (match) {
        const error = new Error(message) as ModelNotLoadedError;
        error.name = "ModelNotLoadedError";
        error.code = "MODEL_NOT_LOADED";
        error.statusCode = status;
        return error;
      }
    }

    const error = new Error(message) as OpenLLMError;
    error.name = "OpenLLMError";
    error.code = "API_ERROR";
    error.statusCode = status;
    return error;
  }

  async health(): Promise<HealthResponse> {
    return this.request<HealthResponse>("/health");
  }

  async listModels(): Promise<ModelListResponse> {
    return this.request<ModelListResponse>("/v1/models");
  }

  async registerModel(
    data: RegisterModelRequest,
  ): Promise<RegisterModelResponse> {
    return this.request<RegisterModelResponse>("/v1/models/register", {
      method: "POST",
      body: JSON.stringify(data),
    });
  }

  async loadModel(data: LoadModelRequest): Promise<LoadModelResponse> {
    return this.request<LoadModelResponse>("/v1/models/load", {
      method: "POST",
      body: JSON.stringify(data),
    });
  }

  async unloadModel(modelId: string): Promise<UnloadModelResponse> {
    return this.request<UnloadModelResponse>(
      `/v1/models/unload/${modelId}`,
      {
        method: "POST",
      },
    );
  }

  async inference(data: InferenceRequest): Promise<InferenceResponse> {
    return this.request<InferenceResponse>("/v1/inference", {
      method: "POST",
      body: JSON.stringify(data),
    });
  }

  async inferenceStream(
    data: InferenceRequest,
    options: StreamOptions,
  ): Promise<void> {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.timeout);

    try {
      const response = await fetch(`${this.baseUrl}/v1/inference/stream`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify(data),
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      if (!response.ok) {
        const errorText = await response.text();
        const error = this.createError(response.status, errorText);
        options.onError?.(error);
        throw error;
      }

      const reader = response.body?.getReader();
      if (!reader) {
        const error = new Error("No response body");
        options.onError?.(error);
        throw error;
      }

      const decoder = new TextDecoder();
      let buffer = "";
      let accumulatedText = "";
      let tokenCount = 0;
      const modelId = data.model_id;

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        const lines = buffer.split("\n");
        buffer = lines.pop() ?? "";

        for (const line of lines) {
          if (line.trim().startsWith("event: token")) {
            continue;
          }
          if (line.trim().startsWith("data: ")) {
            const data = line.trim().slice(6);
            if (data) {
              try {
                const token = JSON.parse(data) as StreamToken;
                accumulatedText += token.token;
                tokenCount++;
                options.onToken(token);

                if (token.complete) {
                  options.onComplete?.({
                    model_id: modelId,
                    text: accumulatedText,
                    tokens_generated: tokenCount,
                    finish_reason: "stop",
                  });
                }
              } catch (e) {
                console.error("Failed to parse SSE data:", data, e);
              }
            }
          }
        }
      }
    } catch (error) {
      clearTimeout(timeoutId);
      if (error instanceof Error) {
        if (error.name === "AbortError") {
          const timeoutError = new Error(
            `Stream timeout after ${this.timeout}ms`,
          );
          options.onError?.(timeoutError);
          throw timeoutError;
        }
        options.onError?.(error);
        throw error;
      }
    }
  }

  setBaseUrl(baseUrl: string): void {
    this.baseUrl = baseUrl;
  }

  getBaseUrl(): string {
    return this.baseUrl;
  }

  setTimeout(timeout: number): void {
    this.timeout = timeout;
  }

  getTimeout(): number {
    return this.timeout;
  }
}

export function createOpenLLMClient(config?: OpenLLMConfig): OpenLLMClient {
  return new OpenLLMClient(config);
}
