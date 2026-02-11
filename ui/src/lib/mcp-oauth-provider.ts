import type { OAuthClientProvider } from "@modelcontextprotocol/sdk/client/auth.js";
import type {
  OAuthClientMetadata,
  OAuthClientInformationMixed,
  OAuthTokens,
} from "@modelcontextprotocol/sdk/shared/auth.js";

const POPUP_WIDTH = 600;
const POPUP_HEIGHT = 700;

/**
 * Detect the callback URL dynamically from the current location.
 * Handles both gateway-proxied access (/ui/playground) and direct dev access (/playground).
 * Looks for "/playground" in the current pathname to find the basePath prefix.
 *
 * IMPORTANT: The trailing slash is required. Without it, the static file server
 * (ServeDir behind nest_service) will redirect to add a trailing slash, but
 * that redirect uses the internal backend origin (stripping the gateway prefix),
 * which breaks the OAuth callback.
 */
function getCallbackUrl(): URL {
  const pathname = window.location.pathname;
  const playgroundIdx = pathname.indexOf("/playground");
  const basePath = playgroundIdx > 0 ? pathname.substring(0, playgroundIdx) : "";
  return new URL(`${window.location.origin}${basePath}/playground/oauth/callback/`);
}

/**
 * Browser-based OAuthClientProvider for the playground.
 *
 * Handles the full MCP OAuth 2.1 flow using:
 * - A popup window for user authorization
 * - postMessage for receiving the authorization code from the callback page
 * - localStorage for persisting tokens and client registration (keyed by server URL)
 * - sessionStorage for PKCE code verifier
 */
export class BrowserOAuthProvider implements OAuthClientProvider {
  private serverUrl: string;
  private _authPromise: Promise<string> | null = null;
  private _authResolve: ((code: string) => void) | null = null;
  private _authReject: ((err: Error) => void) | null = null;
  private _messageHandler: ((event: MessageEvent) => void) | null = null;
  private _popupPollTimer: ReturnType<typeof setInterval> | null = null;

  constructor(serverUrl: string) {
    this.serverUrl = serverUrl;
  }

  private storageKey(suffix: string): string {
    return `mcp_oauth_${suffix}_${this.serverUrl}`;
  }

  get redirectUrl(): URL {
    return getCallbackUrl();
  }

  get clientMetadata(): OAuthClientMetadata {
    return {
      redirect_uris: [this.redirectUrl.href],
      client_name: "AgentGateway Playground",
      grant_types: ["authorization_code", "refresh_token"],
      response_types: ["code"],
      token_endpoint_auth_method: "none",
    } as OAuthClientMetadata;
  }

  clientInformation(): OAuthClientInformationMixed | undefined {
    const stored = localStorage.getItem(this.storageKey("client"));
    if (!stored) return undefined;
    try {
      return JSON.parse(stored) as OAuthClientInformationMixed;
    } catch {
      return undefined;
    }
  }

  saveClientInformation(clientInformation: OAuthClientInformationMixed): void {
    localStorage.setItem(this.storageKey("client"), JSON.stringify(clientInformation));
  }

  tokens(): OAuthTokens | undefined {
    const stored = localStorage.getItem(this.storageKey("tokens"));
    if (!stored) return undefined;
    try {
      return JSON.parse(stored) as OAuthTokens;
    } catch {
      return undefined;
    }
  }

  saveTokens(tokens: OAuthTokens): void {
    localStorage.setItem(this.storageKey("tokens"), JSON.stringify(tokens));
  }

  saveCodeVerifier(codeVerifier: string): void {
    sessionStorage.setItem(this.storageKey("verifier"), codeVerifier);
  }

  codeVerifier(): string {
    const verifier = sessionStorage.getItem(this.storageKey("verifier"));
    if (!verifier) {
      throw new Error("No code verifier saved");
    }
    return verifier;
  }

  redirectToAuthorization(authorizationUrl: URL): void {
    // Set up promise to receive the auth code from the popup
    this._authPromise = new Promise<string>((resolve, reject) => {
      this._authResolve = resolve;
      this._authReject = reject;
    });

    // Listen for postMessage from the callback page
    this._messageHandler = (event: MessageEvent) => {
      if (event.origin !== window.location.origin) return;
      if (event.data?.type !== "mcp-oauth-callback") return;

      this.cleanupPopupWatcher();

      if (event.data.error) {
        this._authReject?.(new Error(`OAuth authorization failed: ${event.data.error}`));
      } else if (event.data.code) {
        this._authResolve?.(event.data.code);
      } else {
        this._authReject?.(new Error("No authorization code received from callback"));
      }
    };
    window.addEventListener("message", this._messageHandler);

    // Open popup centered on screen
    const left = Math.round(window.screenX + (window.outerWidth - POPUP_WIDTH) / 2);
    const top = Math.round(window.screenY + (window.outerHeight - POPUP_HEIGHT) / 2);
    const popup = window.open(
      authorizationUrl.toString(),
      "mcp_oauth",
      `width=${POPUP_WIDTH},height=${POPUP_HEIGHT},left=${left},top=${top},popup=true`
    );

    // Detect popup blocked
    if (!popup || popup.closed) {
      this.cleanupPopupWatcher();
      this._authReject?.(
        new Error(
          "Popup was blocked by the browser. Please allow popups for this site and try again."
        )
      );
      return;
    }

    // Poll for popup being closed manually (user cancelled)
    this._popupPollTimer = setInterval(() => {
      if (popup.closed) {
        this.cleanupPopupWatcher();
        this._authReject?.(new Error("Authorization cancelled — popup was closed."));
      }
    }, 500);
  }

  /**
   * Wait for the authorization code from the popup callback.
   * This is called by the playground after catching UnauthorizedError.
   */
  waitForAuthCode(): Promise<string> {
    if (!this._authPromise) {
      return Promise.reject(new Error("No OAuth authorization flow in progress"));
    }
    return this._authPromise;
  }

  /**
   * Clear all stored OAuth data for this server.
   */
  clearAuth(): void {
    localStorage.removeItem(this.storageKey("client"));
    localStorage.removeItem(this.storageKey("tokens"));
    sessionStorage.removeItem(this.storageKey("verifier"));
  }

  /**
   * Clean up event listeners.
   */
  dispose(): void {
    this.cleanupPopupWatcher();
  }

  invalidateCredentials(scope: "all" | "client" | "tokens" | "verifier"): void {
    switch (scope) {
      case "all":
        this.clearAuth();
        break;
      case "client":
        localStorage.removeItem(this.storageKey("client"));
        break;
      case "tokens":
        localStorage.removeItem(this.storageKey("tokens"));
        break;
      case "verifier":
        sessionStorage.removeItem(this.storageKey("verifier"));
        break;
    }
  }

  private cleanupPopupWatcher(): void {
    if (this._messageHandler) {
      window.removeEventListener("message", this._messageHandler);
      this._messageHandler = null;
    }
    if (this._popupPollTimer) {
      clearInterval(this._popupPollTimer);
      this._popupPollTimer = null;
    }
  }
}
