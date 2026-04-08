#!/bin/bash
# Vercel 전용 통합 빌드 스크립트
# 사용법: Vercel 내 Build Command를 `bash build_for_vercel.sh` 로 지정합니다.

echo "==== 1. wasm-pack 다운로드 및 설치 ===="
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
# Vercel이 패키지를 설치한 경로를 시스템 환경 변수에 강제로 병합합니다.
export PATH="/rust/bin:$HOME/.cargo/bin:$PATH"

echo "==== 2. Rust 엔진 WebAssembly 컴파일 ===="
cd chart_engine
wasm-pack build --target web --out-dir ../web/pkg

echo "==== 3. 프론트엔드 모듈 설치 및 최종 빌드 ===="
cd ../web
npm install
npm run build
