import { createOpenLLMClient } from "@use-solace/openllm";

async function main() {
  const client = createOpenLLMClient({ engine: 8080 });

  try {
    console.log("1. Registering mistral model...");
    const registerResponse = await client.registerModel({
      id: "mistral",
      name: "Mistral 7B",
      inference: "ollama",
      context: 8192,
      quant: "Q4_K_M",
      capabilities: ["chat", "completion"],
      latency: "fast",
    });
    console.log("   Registered:", registerResponse.model.name);

    console.log("\n2. Loading mistral model...");
    const loadResponse = await client.loadModel({ model_id: "mistral" });
    console.log("   Load response:", loadResponse.message);

    console.log("\n3. Running streamed inference...");
    console.log("   Prompt: 'What is the meaning of life?'");
    console.log("\n   Response:");

    let fullResponse = "";

    await client.inferenceStream(
      {
        model_id: "mistral",
        prompt: "What is the meaning of life?",
        max_tokens: 512,
        temperature: 0.7,
      },
      {
        onToken: (token) => {
          process.stdout.write(token.token);
          fullResponse += token.token;
        },
        onComplete: (response) => {
          console.log("\n\n   Stream completed!");
          console.log("   Total tokens:", response.tokens_generated);
        },
        onError: (error) => {
          console.error("\n   Stream error:", error.message);
        },
      },
    );

    console.log("\n\n4. Unloading model...");
    const unloadResponse = await client.unloadModel("mistral");
    console.log("   Unload response:", unloadResponse.message);

  } catch (error) {
    if (error instanceof Error) {
      console.error("Error:", error.message);
    }
  }
}

main();
