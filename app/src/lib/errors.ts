const FRIENDLY_ERRORS: Array<[RegExp, string]> = [
  [/MaxTransferAmountExceeded/i, "Transfer rejected: maximum transfer amount exceeded."],
  [/DailyTransferLimitExceeded/i, "Transfer rejected: daily transfer limit exceeded."],
  [/Sender.*Unauthorized|SenderNotAuthorized/i, "Transfer rejected: sender is not authorized."],
  [/Receiver.*Unauthorized|ReceiverNotAuthorized/i, "Transfer rejected: receiver is not authorized."],
  [/Blocked/i, "Transfer rejected: one of the wallets is blocked."],
  [/Policy.*Disabled/i, "Transfer rejected: compliance policy is disabled."],
  [/Policy.*Expired/i, "Transfer rejected: compliance policy has expired."],
  [/0x177a/i, "Transfer rejected: maximum transfer amount exceeded."]
];

export function explainError(error: unknown) {
  const text =
    error instanceof Error
      ? `${error.message}\n${(error as any).logs?.join?.("\n") ?? ""}`
      : String(error);

  for (const [pattern, message] of FRIENDLY_ERRORS) {
    if (pattern.test(text)) return message;
  }

  const anchorMatch = text.match(
    /Error Code:\s*([A-Za-z0-9_]+).*?Error Message:\s*([^"\n]+)/s
  );

  if (anchorMatch) {
    return `${anchorMatch[1]}: ${anchorMatch[2].trim()}`;
  }

  return text.length > 280 ? `${text.slice(0, 280)}…` : text;
}
