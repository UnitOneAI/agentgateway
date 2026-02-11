"use client";

import { Suspense, useEffect, useState } from "react";
import { useSearchParams } from "next/navigation";

/**
 * Inner component that uses useSearchParams (must be wrapped in Suspense).
 *
 * Extracts the authorization code (or error) from URL search params,
 * sends it back to the opener window via postMessage, and closes itself.
 */
function OAuthCallbackContent() {
  const searchParams = useSearchParams();
  const [status, setStatus] = useState<"processing" | "success" | "error" | "no-opener">(
    "processing"
  );
  const [errorMessage, setErrorMessage] = useState<string>("");

  useEffect(() => {
    const code = searchParams.get("code");
    const error = searchParams.get("error");
    const errorDescription = searchParams.get("error_description");

    if (!window.opener) {
      setStatus("no-opener");
      return;
    }

    if (error) {
      setStatus("error");
      setErrorMessage(errorDescription || error);
      window.opener.postMessage(
        { type: "mcp-oauth-callback", code: null, error: errorDescription || error },
        window.location.origin
      );
      setTimeout(() => window.close(), 3000);
      return;
    }

    if (code) {
      setStatus("success");
      window.opener.postMessage(
        { type: "mcp-oauth-callback", code, error: null },
        window.location.origin
      );
      setTimeout(() => window.close(), 1000);
      return;
    }

    setStatus("error");
    setErrorMessage("No authorization code or error received.");
    window.opener.postMessage(
      { type: "mcp-oauth-callback", code: null, error: "No authorization code received" },
      window.location.origin
    );
  }, [searchParams]);

  return (
    <div className="text-center space-y-4 p-8">
      {status === "processing" && (
        <>
          <div className="animate-spin h-8 w-8 border-4 border-primary border-t-transparent rounded-full mx-auto" />
          <p className="text-muted-foreground">Completing authentication...</p>
        </>
      )}
      {status === "success" && (
        <>
          <div className="text-green-600 text-4xl">&#10003;</div>
          <p className="font-medium">Authorization successful!</p>
          <p className="text-sm text-muted-foreground">This window will close automatically.</p>
        </>
      )}
      {status === "error" && (
        <>
          <div className="text-red-600 text-4xl">&#10007;</div>
          <p className="font-medium">Authorization failed</p>
          <p className="text-sm text-muted-foreground">{errorMessage}</p>
          <p className="text-xs text-muted-foreground">This window will close shortly.</p>
        </>
      )}
      {status === "no-opener" && (
        <>
          <div className="text-orange-600 text-4xl">&#9888;</div>
          <p className="font-medium">Unable to complete authorization</p>
          <p className="text-sm text-muted-foreground">
            This page should be opened from the playground. Please close this window and try again.
          </p>
        </>
      )}
    </div>
  );
}

/**
 * OAuth callback page for the playground.
 * Opened in a popup by the BrowserOAuthProvider during the MCP OAuth flow.
 */
export default function OAuthCallbackPage() {
  return (
    <div className="flex items-center justify-center min-h-screen bg-background">
      <Suspense
        fallback={
          <div className="text-center space-y-4 p-8">
            <div className="animate-spin h-8 w-8 border-4 border-primary border-t-transparent rounded-full mx-auto" />
            <p className="text-muted-foreground">Completing authentication...</p>
          </div>
        }
      >
        <OAuthCallbackContent />
      </Suspense>
    </div>
  );
}
