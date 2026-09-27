export type GlobalPolicyView = {
  admin: any;
  mint: any;
  enabled: boolean;
  maxTransferAmount: any;
  dailyTransferLimit: any;
  policyVersion: any;
  expiresAt: any;
  bump: number;
};

export type AuthorizationView = {
  mint: any;
  wallet: any;
  status: unknown;
  bump: number;
};

export type TransferStatsView = {
  mint: any;
  wallet: any;
  dayIndex: any;
  amountToday: any;
  totalTransferred: any;
  transferCount: any;
  bump: number;
};
