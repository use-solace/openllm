import type {
  FindModelOptions,
  ModelRegistryConfig,
  ModelRegistryEntry,
  RegistryEntryInput,
} from "./types.js";

export class ModelRegistryImpl {
  private entries: Map<string, ModelRegistryEntry> = new Map();

  constructor(config: ModelRegistryConfig) {
    const entries = config.entries ?? {};
    for (const [id, entry] of Object.entries(entries)) {
      this.registerEntry(id, entry);
    }
  }

  private registerEntry(id: string, entry: RegistryEntryInput): ModelRegistryEntry {
    const model: ModelRegistryEntry = {
      id,
      name: entry.id,
      inference: entry.inference,
      context: entry.context,
      quant: entry.quant,
      capabilities: entry.capabilities,
      latency: entry.latency,
      size_bytes: 4_000_000_000,
      loaded: false,
      loaded_at: undefined,
    };
    this.entries.set(id, model);
    return model;
  }

  list(): ModelRegistryEntry[] {
    return Array.from(this.entries.values());
  }

  get(id: string): ModelRegistryEntry | undefined {
    return this.entries.get(id);
  }

  find(options: FindModelOptions = {}): ModelRegistryEntry[] {
    const results = this.list().filter((model) => {
      if (options.capability && !model.capabilities.includes(options.capability)) {
        return false;
      }
      if (options.latency && model.latency !== options.latency) {
        return false;
      }
      if (options.inference && model.inference !== options.inference) {
        return false;
      }
      if (options.minContext && model.context < options.minContext) {
        return false;
      }
      if (options.loaded !== undefined && model.loaded !== options.loaded) {
        return false;
      }
      return true;
    });
    return results;
  }

  findOne(options: FindModelOptions = {}): ModelRegistryEntry | undefined {
    return this.find(options)[0];
  }

  has(id: string): boolean {
    return this.entries.has(id);
  }

  count(): number {
    return this.entries.size;
  }

  add(id: string, entry: RegistryEntryInput): ModelRegistryEntry {
    if (this.entries.has(id)) {
      throw new Error(`Model with id '${id}' already exists`);
    }
    return this.registerEntry(id, entry);
  }

  update(id: string, updates: Partial<RegistryEntryInput>): ModelRegistryEntry {
    const existing = this.entries.get(id);
    if (!existing) {
      throw new Error(`Model with id '${id}' not found`);
    }

    const updated: ModelRegistryEntry = {
      ...existing,
      ...updates,
      id,
    };
    this.entries.set(id, updated);
    return updated;
  }

  remove(id: string): boolean {
    return this.entries.delete(id);
  }

  clear(): void {
    this.entries.clear();
  }

  toObject(): Record<string, ModelRegistryEntry> {
    return Object.fromEntries(this.entries);
  }

  fromObject(obj: Record<string, ModelRegistryEntry>): void {
    this.entries.clear();
    for (const [id, entry] of Object.entries(obj)) {
      this.entries.set(id, entry);
    }
  }
}

export function ModelRegistry(
  config: ModelRegistryConfig,
): ModelRegistryImpl {
  return new ModelRegistryImpl(config);
}
