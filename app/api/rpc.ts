import type {
  VercelRequest,
  VercelResponse,
} from "@vercel/node";

export default async function handler(
  req: VercelRequest,
  res: VercelResponse
) {
  if (req.method !== "POST") {
    res.setHeader("Allow", "POST");

    return res
      .status(405)
      .json({
        error: "Method not allowed",
      });
  }

  const upstream =
    process.env.HELIUS_DEVNET_RPC;

  if (!upstream) {
    return res
      .status(500)
      .json({
        error:
          "HELIUS_DEVNET_RPC is not configured",
      });
  }

  try {
    const body =
      typeof req.body === "string"
        ? req.body
        : JSON.stringify(req.body);

    const response =
      await fetch(upstream, {
        method: "POST",

        headers: {
          "content-type":
            "application/json",
        },

        body,
      });

    const text =
      await response.text();

    res.setHeader(
      "content-type",
      response.headers.get(
        "content-type"
      ) ??
        "application/json"
    );

    res.setHeader(
      "cache-control",
      "no-store"
    );

    return res
      .status(response.status)
      .send(text);
  } catch (error) {
    console.error(
      "RPC proxy error:",
      error
    );

    return res
      .status(502)
      .json({
        error:
          "Unable to reach Solana RPC",
      });
  }
}