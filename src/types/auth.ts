export type AccountType = "microsoft" | "dev";

export interface AccountProfile {
  id: string;
  account_type: AccountType;
  uuid: string;
  username: string;
  skin_url?: string | null;
  is_active: boolean;
}

export interface AuthStatePayload {
  accounts: AccountProfile[];
  active_account?: AccountProfile | null;
}

export interface DeviceCodeResponse {
  device_code: string;
  user_code: string;
  verification_uri: string;
  expires_in: number;
  interval: number;
  message?: string;
}
