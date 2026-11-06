import React, { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AccountProfile, DeviceCodeResponse } from "../../types/auth";
import { X, Globe, Smartphone, Copy, Check, ExternalLink, Loader2, AlertCircle } from "lucide-react";

interface LoginModalProps {
  isOpen: boolean;
  onClose: () => void;
  onLoginSuccess: (account: AccountProfile) => void;
}

export const LoginModal: React.FC<LoginModalProps> = ({
  isOpen,
  onClose,
  onLoginSuccess,
}) => {
  const [activeTab, setActiveTab] = useState<"loopback" | "devicecode">("loopback");
  const [isLoopbackWaiting, setIsLoopbackWaiting] = useState(false);
  const [deviceCodeData, setDeviceCodeData] = useState<DeviceCodeResponse | null>(null);
  const [isDeviceCodePolling, setIsDeviceCodePolling] = useState(false);
  const [isCopied, setIsCopied] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const pollingRef = useRef<number | null>(null);

  const stopDevicePolling = useCallback(() => {
    if (pollingRef.current) {
      clearInterval(pollingRef.current);
      pollingRef.current = null;
    }
    setIsDeviceCodePolling(false);
  }, []);

  // Stop polling and reset state on modal close or unmount
  useEffect(() => {
    if (!isOpen) {
      stopDevicePolling();
      setIsLoopbackWaiting(false);
      setDeviceCodeData(null);
      setErrorMessage(null);
    }
    return () => {
      stopDevicePolling();
    };
  }, [isOpen, stopDevicePolling]);

  if (!isOpen) return null;

  // 1. Handle Loopback Flow
  const handleStartLoopback = async () => {
    try {
      setErrorMessage(null);
      setIsLoopbackWaiting(true);
      const profile = await invoke<AccountProfile>("start_login_loopback");
      setIsLoopbackWaiting(false);
      onLoginSuccess(profile);
      onClose();
    } catch (err) {
      setIsLoopbackWaiting(false);
      setErrorMessage(String(err));
    }
  };

  // 2. Handle Device Code Flow
  const handleStartDeviceCode = async () => {
    try {
      setErrorMessage(null);
      stopDevicePolling();

      const data = await invoke<DeviceCodeResponse>("start_login_device_code");
      setDeviceCodeData(data);
      setIsDeviceCodePolling(true);

      const intervalMs = Math.max((data.interval || 5) * 1000, 3000);
      pollingRef.current = window.setInterval(async () => {
        try {
          const profile = await invoke<AccountProfile | null>("poll_login_device_code", {
            deviceCode: data.device_code,
          });
          if (profile) {
            stopDevicePolling();
            onLoginSuccess(profile);
            onClose();
          }
        } catch (pollErr) {
          stopDevicePolling();
          setErrorMessage(String(pollErr));
        }
      }, intervalMs);
    } catch (err) {
      setErrorMessage(String(err));
    }
  };

  const handleCopyCode = async () => {
    if (!deviceCodeData) return;
    try {
      await navigator.clipboard.writeText(deviceCodeData.user_code);
      setIsCopied(true);
      setTimeout(() => setIsCopied(false), 2000);
    } catch {
      // ignore clipboard write failure
    }
  };

  const openLink = async (url: string) => {
    try {
      await invoke("open_external_url", { url });
    } catch {
      window.open(url, "_blank");
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4 animate-in fade-in duration-200 select-none">
      <div className="w-full max-w-md bg-szz-surface-0 border border-szz-border rounded-2xl shadow-2xl overflow-hidden flex flex-col text-szz-text">
        {/* Header */}
        <div className="px-6 py-4 border-b border-szz-border flex items-center justify-between">
          <div className="flex items-center gap-2.5">
            <div className="w-8 h-8 rounded-lg bg-szz-matcha/15 text-szz-matcha flex items-center justify-center border border-szz-matcha/30">
              <Globe className="w-4 h-4" />
            </div>
            <div>
              <h3 className="text-base font-bold text-szz-text">Sign In with Microsoft</h3>
              <p className="text-xs text-szz-subtext-0">Connect your Minecraft Java Edition account</p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="text-szz-subtext-0 hover:text-szz-text p-1 rounded-lg hover:bg-szz-surface-1 transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Tab Switcher */}
        <div className="flex border border-szz-border bg-szz-surface-1/60 p-1 m-4 mb-2 rounded-xl">
          <button
            onClick={() => {
              setActiveTab("loopback");
              stopDevicePolling();
              setErrorMessage(null);
            }}
            className={`flex-1 flex items-center justify-center gap-2 py-2 text-xs font-semibold rounded-lg transition-all ${
              activeTab === "loopback"
                ? "bg-szz-surface-0 text-szz-text shadow-sm border border-szz-border"
                : "text-szz-subtext-0 hover:text-szz-text"
            }`}
          >
            <Globe className="w-3.5 h-3.5" />
            <span>Browser Login (Standard)</span>
          </button>

          <button
            onClick={() => {
              setActiveTab("devicecode");
              setErrorMessage(null);
              if (!deviceCodeData) {
                handleStartDeviceCode();
              }
            }}
            className={`flex-1 flex items-center justify-center gap-2 py-2 text-xs font-semibold rounded-lg transition-all ${
              activeTab === "devicecode"
                ? "bg-szz-surface-0 text-szz-text shadow-sm border border-szz-border"
                : "text-szz-subtext-0 hover:text-szz-text"
            }`}
          >
            <Smartphone className="w-3.5 h-3.5" />
            <span>Device Code (Fallback)</span>
          </button>
        </div>

        {/* Error Alert */}
        {errorMessage && (
          <div className="mx-6 mt-2 p-3 rounded-xl bg-szz-berry/15 border border-szz-berry/30 flex items-start gap-2.5 text-xs text-szz-berry">
            <AlertCircle className="w-4 h-4 text-szz-berry shrink-0 mt-0.5" />
            <div className="flex-1 break-words">{errorMessage}</div>
          </div>
        )}

        {/* Tab 1: Browser Loopback Content */}
        {activeTab === "loopback" && (
          <div className="p-6 pt-3 flex flex-col items-center text-center">
            {!isLoopbackWaiting ? (
              <>
                <p className="text-xs text-szz-subtext-0 leading-relaxed mb-6">
                  SZZ will open your default web browser to the official Microsoft authentication page.
                  After signing in, your browser will automatically redirect back to the launcher.
                </p>

                <button
                  onClick={handleStartLoopback}
                  className="w-full py-3 px-4 bg-szz-matcha hover:opacity-90 text-szz-actionPlayText font-bold text-sm rounded-xl transition-all shadow-lg shadow-szz-matcha/20 flex items-center justify-center gap-2 cursor-pointer"
                >
                  <ExternalLink className="w-4 h-4" />
                  <span>Open Browser to Sign In</span>
                </button>
              </>
            ) : (
              <div className="py-8 flex flex-col items-center">
                <div className="w-16 h-16 rounded-full bg-szz-matcha/15 border border-szz-matcha/30 flex items-center justify-center animate-pulse mb-5">
                  <Loader2 className="w-8 h-8 text-szz-matcha animate-spin" />
                </div>
                <h4 className="text-sm font-bold text-szz-text mb-1">Waiting for Authorization...</h4>
                <p className="text-xs text-szz-subtext-0 max-w-xs mb-6">
                  Complete the sign-in prompt in your browser window. SZZ is listening on localhost for the redirect.
                </p>
                <button
                  onClick={() => setIsLoopbackWaiting(false)}
                  className="px-4 py-1.5 text-xs text-szz-subtext-0 hover:text-szz-text border border-szz-border rounded-lg hover:bg-szz-surface-1"
                >
                  Cancel
                </button>
              </div>
            )}
          </div>
        )}

        {/* Tab 2: Device Code Content */}
        {activeTab === "devicecode" && (
          <div className="p-6 pt-3 flex flex-col items-center text-center">
            {!deviceCodeData ? (
              <div className="py-8 flex flex-col items-center">
                <Loader2 className="w-6 h-6 text-szz-matcha animate-spin mb-2" />
                <span className="text-xs text-szz-subtext-0">Microsoft로부터 기기 인증 코드를 요청하는 중...</span>
              </div>
            ) : (
              <>
                <div className="w-full text-left bg-szz-surface-1/60 border border-szz-border rounded-xl p-3.5 mb-4 text-xs space-y-2">
                  <div className="flex items-start gap-2">
                    <span className="w-4 h-4 rounded-full bg-szz-matcha/20 text-szz-matcha font-bold flex items-center justify-center shrink-0 text-[10px]">1</span>
                    <span className="text-szz-subtext-0">
                      브라우저(스마트폰 또는 PC)에서 <span className="text-szz-text font-mono font-semibold">microsoft.com/link</span> 로 이동합니다.
                    </span>
                  </div>
                  <div className="flex items-start gap-2">
                    <span className="w-4 h-4 rounded-full bg-szz-matcha/20 text-szz-matcha font-bold flex items-center justify-center shrink-0 text-[10px]">2</span>
                    <span className="text-szz-subtext-0">
                      아래의 <span className="text-szz-matcha font-semibold">8자리 인증 코드</span>를 입력하고 MS 계정으로 로그인합니다.
                    </span>
                  </div>
                  <div className="flex items-start gap-2">
                    <span className="w-4 h-4 rounded-full bg-szz-matcha/20 text-szz-matcha font-bold flex items-center justify-center shrink-0 text-[10px]">3</span>
                    <span className="text-szz-subtext-0">
                      브라우저에서 완료하면 이 창에서 <span className="text-szz-text font-semibold">자동으로 로그인이 완료</span>됩니다!
                    </span>
                  </div>
                </div>

                <div className="w-full bg-szz-surface-1 border border-szz-border rounded-xl p-4 mb-4 flex items-center justify-between">
                  <span className="text-2xl font-mono font-bold tracking-widest text-szz-matcha">
                    {deviceCodeData.user_code}
                  </span>
                  <button
                    onClick={handleCopyCode}
                    className="flex items-center gap-1.5 px-3 py-1.5 bg-szz-surface-2 hover:bg-szz-surface-0 text-xs font-semibold rounded-lg text-szz-text border border-szz-border transition-colors cursor-pointer"
                  >
                    {isCopied ? (
                      <>
                        <Check className="w-3.5 h-3.5 text-szz-matcha" />
                        <span>복사됨!</span>
                      </>
                    ) : (
                      <>
                        <Copy className="w-3.5 h-3.5" />
                        <span>코드 복사</span>
                      </>
                    )}
                  </button>
                </div>

                <button
                  onClick={() => openLink(deviceCodeData.verification_uri)}
                  className="w-full py-2.5 px-3 mb-4 bg-szz-surface-2 hover:bg-szz-surface-1 border border-szz-matcha text-szz-matcha font-semibold text-xs rounded-xl transition-all flex items-center justify-center gap-1.5 cursor-pointer shadow-sm"
                >
                  <ExternalLink className="w-3.5 h-3.5" />
                  <span>브라우저 열기 (microsoft.com/link)</span>
                </button>

                {isDeviceCodePolling && (
                  <div className="flex items-center gap-2 text-xs text-szz-subtext-0">
                    <Loader2 className="w-3.5 h-3.5 animate-spin text-szz-matcha" />
                    <span>브라우저에서 로그인이 완료되기를 기다리는 중...</span>
                  </div>
                )}
              </>
            )}
          </div>
        )}
      </div>
    </div>
  );
};
