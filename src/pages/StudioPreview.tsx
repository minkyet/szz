import React from "react";
import { ArrowLeft, Sparkles, UploadCloud, BarChart3, BellRing } from "lucide-react";

export interface StudioPreviewProps {
  onBack: () => void;
}

export const StudioPreview: React.FC<StudioPreviewProps> = ({ onBack }) => {
  return (
    <div className="flex-1 overflow-y-auto px-6 py-10 flex flex-col items-center justify-center font-pixel text-szz-text">
      <div className="max-w-xl w-full flex flex-col items-center text-center gap-6">
        {/* Back navigation button */}
        <button
          type="button"
          onClick={onBack}
          className="self-start flex items-center gap-1.5 text-xs text-szz-subtext-0 hover:text-szz-text transition-colors cursor-pointer group"
        >
          <ArrowLeft className="w-3.5 h-3.5 group-hover:-translate-x-0.5 transition-transform" />
          <span>웹 카탈로그로 돌아가기</span>
        </button>

        {/* Studio Badge */}
        <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-szz-amber/10 border border-szz-amber/30 text-szz-amber text-xs font-semibold">
          <Sparkles className="w-3.5 h-3.5" />
          <span>Creator Studio • Coming in Milestone W3</span>
        </div>

        {/* Headline */}
        <div className="flex flex-col gap-2">
          <h1 className="text-3xl font-extrabold tracking-tight text-szz-text">
            나만의 마인크래프트 서버를 <span className="text-szz-amber">원클릭으로 등록</span>하세요
          </h1>
          <p className="text-sm text-szz-subtext-0 leading-relaxed">
            유튜브 채널을 개설하듯 서버 배너, 모드팩, 소개글을 등록하고
            유저들이 단 한 번의 클릭으로 서버에 접속할 수 있도록 공유하세요.
          </p>
        </div>

        {/* Coming soon feature list */}
        <div className="flex flex-col gap-3 w-full text-left mt-2">
          <div className="p-4 rounded-xl bg-szz-surface-0/60 border border-szz-border flex items-start gap-3">
            <div className="p-2 rounded-lg bg-szz-surface-1 border border-szz-border text-szz-amber shrink-0">
              <UploadCloud className="w-4 h-4" />
            </div>
            <div>
              <h3 className="text-xs font-bold text-szz-text">웹 기반 모드팩 자동 변환 업로더</h3>
              <p className="text-[11px] text-szz-subtext-0 mt-0.5">
                `.mrpack` 또는 `.zip` 모드팩 파일을 브라우저에 드래그하면 CAS 클라우드 스토리지에 자동 최적화 등록됩니다.
              </p>
            </div>
          </div>

          <div className="p-4 rounded-xl bg-szz-surface-0/60 border border-szz-border flex items-start gap-3">
            <div className="p-2 rounded-lg bg-szz-surface-1 border border-szz-border text-szz-matcha shrink-0">
              <BarChart3 className="w-4 h-4" />
            </div>
            <div>
              <h3 className="text-xs font-bold text-szz-text">실시간 동접자 및 핑 통계 모니터링</h3>
              <p className="text-[11px] text-szz-subtext-0 mt-0.5">
                SLP 크론 워커가 서버 상태와 동접자를 1분마다 측정하여 실시간 차트로 제공합니다.
              </p>
            </div>
          </div>

          <div className="p-4 rounded-xl bg-szz-surface-0/60 border border-szz-border flex items-start gap-3">
            <div className="p-2 rounded-lg bg-szz-surface-1 border border-szz-border text-szz-bamboo shrink-0">
              <BellRing className="w-4 h-4" />
            </div>
            <div>
              <h3 className="text-xs font-bold text-szz-text">패치노트 발행 & 증분 업데이트 배포</h3>
              <p className="text-[11px] text-szz-subtext-0 mt-0.5">
                모드 추가/삭제 변경 사항을 발행하면 유저의 런처에 변경된 모드만 즉시 증분 업데이트됩니다.
              </p>
            </div>
          </div>
        </div>

        {/* Back Button */}
        <button
          type="button"
          onClick={onBack}
          className="mt-2 px-6 py-2.5 rounded-xl bg-szz-surface-1 hover:bg-szz-surface-2 border border-szz-border text-xs font-bold text-szz-text transition-all cursor-pointer"
        >
          서버 목록 둘러보기
        </button>
      </div>
    </div>
  );
};
