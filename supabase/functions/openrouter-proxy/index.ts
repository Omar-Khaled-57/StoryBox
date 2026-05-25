import { serve } from "https://deno.land/std@0.177.0/http/server.ts"

const OPENROUTER_API_KEY = Deno.env.get("OPENROUTER_API_KEY")
const OPENROUTER_BASE = "https://openrouter.ai/api/v1"

serve(async (req) => {
  if (req.method !== "POST") {
    return new Response("Method not allowed", { status: 405 })
  }

  if (!OPENROUTER_API_KEY) {
    return new Response(JSON.stringify({ error: "OPENROUTER_API_KEY not configured on server" }), {
      status: 500,
      headers: { "Content-Type": "application/json" },
    })
  }

  try {
    const body = await req.json()
    const endpoint = body._endpoint || "chat/completions"
    delete body._endpoint

    const isModelsEndpoint = endpoint === "models"
    const response = await fetch(`${OPENROUTER_BASE}/${endpoint}`, {
      method: isModelsEndpoint ? "GET" : "POST",
      headers: {
        "Authorization": `Bearer ${OPENROUTER_API_KEY}`,
        "Content-Type": "application/json",
        "HTTP-Referer": req.headers.get("origin") || "https://storybox3.app",
        "X-Title": "StoryBox3",
      },
      body: isModelsEndpoint ? undefined : JSON.stringify(body),
    })

    const data = await response.json()
    return new Response(JSON.stringify(data), {
      status: response.status,
      headers: { "Content-Type": "application/json" },
    })
  } catch (err) {
    return new Response(JSON.stringify({ error: err.message }), {
      status: 500,
      headers: { "Content-Type": "application/json" },
    })
  }
})
