# 개인 가계부 (로컬 우선 MVP)

Rust 백엔드와 React PWA 프론트엔드로 구성된 개인 가계부/자산 관리 애플리케이션입니다.

## 주요 기능
- 관리자 비밀번호 로그인
- 거래 CRUD (수입/지출/이체/카드결제)
- 계좌/카테고리/카드 관리
- 월간 대시보드, 캘린더, 월별 정산
- 월간 대시보드 상세 분해(카테고리/카드/계좌) 및 전월 대비
- 월별 정산 차트(카테고리 막대, 카드 비중 도넛)
- 카드 실적 및 혜택 요약
- 카드 상세 페이지(월간 이용 내역/혜택 사용률)
- 파일/텍스트 가져오기 미리보기 및 중복 감지
- 자산 스냅샷 및 순자산 추이
- 모바일 하단 내비게이션 + PWA 설치 지원

## 기술 스택
- 백엔드: Rust, axum, tokio, sqlx, PostgreSQL, notify
- 프론트엔드: React, TypeScript, Vite, TanStack Query, Tailwind CSS, Vite PWA
- 배포: Docker Compose

## 프로젝트 구조
```text
ledger-app/
  backend/
  frontend/
  docker-compose.yml
  .env.example
  README.md
```

## 로컬 실행 방법 (개발)
1. 환경변수 파일 생성
```bash
cp .env.example .env
```

2. 가져오기 폴더 생성
```bash
mkdir -p data/imports
```

3. PostgreSQL 실행 (로컬 또는 Docker)

4. 백엔드 실행
```bash
cd backend
cargo run
```

5. 프론트엔드 실행
```bash
cd frontend
npm install
npm run dev
```

## Docker Compose 실행 방법 (macOS / Linux)
1. 환경변수 준비
```bash
cp .env.example .env
mkdir -p data/imports
```

2. 실행
```bash
docker compose up --build
```

3. 접속
- 프론트엔드: `http://localhost:5173`
- 백엔드: `http://localhost:8080`

## Docker Compose 실행 방법 (Windows)

> **사전 요구사항**: [Docker Desktop for Windows](https://docs.docker.com/desktop/install/windows-install/) 설치 필요

**PowerShell** 기준:

1. 환경변수 준비
```powershell
Copy-Item .env.example .env
New-Item -ItemType Directory -Force data\imports
```

2. `.env` 파일을 메모장이나 VSCode로 열어 `ADMIN_PASSWORD` 등 설정

3. 실행
```powershell
docker compose up --build
```

4. 접속
- 프론트엔드: `http://localhost:5173`
- 백엔드: `http://localhost:8080`

**주의사항 (Windows)**
- Docker Desktop이 실행 중인지 확인 (시스템 트레이 고래 아이콘)
- WSL2 백엔드 사용 권장 (Docker Desktop 설정 → General → "Use WSL 2 based engine" 체크)
- 포트 5173, 8080, 5432가 방화벽에 막혀 있으면 허용 필요
- 경로 구분자는 `\` 대신 `/`를 사용해도 Docker에서는 정상 동작함

**프로젝트 이전 후 재실행 시 (모든 OS 공통)**
```bash
# node_modules, target 등 빌드 캐시는 Docker 빌드 시 자동 생성되므로 없어도 됩니다.
docker compose up --build
```

## 환경변수 설명
- `DATABASE_URL`: PostgreSQL 연결 문자열
- `ADMIN_PASSWORD`: 초기 관리자 비밀번호
- `SESSION_COOKIE_NAME`: 세션 쿠키 이름
- `COOKIE_SECURE`: HTTPS 환경에서 `true` 권장
- `LEDGER_IMPORT_DIR`: 로컬 가져오기 감시 폴더
- `FRONTEND_ORIGIN`: CORS 허용 프론트 주소

## 관리자 비밀번호 설정
- 초기 비밀번호는 `.env`의 `ADMIN_PASSWORD` 값입니다.
- 첫 실행 시 기본 사용자(관리자)가 생성됩니다.
- 비밀번호 변경 시 서버를 재시작하면 적용됩니다.

## 파일 가져오기 폴더 설명
- 기본 경로: `./data/imports`
- 백엔드는 해당 폴더를 감시하여 새 파일이 들어오면 가져오기 미리보기를 생성합니다.
- 자동 저장은 하지 않으며, 웹 UI에서 반드시 `저장하기`를 눌러야 거래로 반영됩니다.

## 지원되는 가져오기 형식
- 업로드: CSV, XLS, XLSX
- 붙여넣기: CSV 유사 텍스트
- 1차 파서:
  - 신한은행 거래내역 CSV
  - 신한카드 XLS/XLSX
  - 일반 붙여넣기 CSV 텍스트

## 샘플 데이터
첫 실행 시 아래 데이터가 자동 생성됩니다.
- 기본 사용자: `관리자`
- 기본 카테고리: 식비, 카페, 교통, 쇼핑, 생활, 고정비, 의료, 문화, 기타, 급여, 이체
- 기본 계좌: 신한은행 입출금, 현금, 카드 미청구금
- 카드사 프리셋 샘플: 신한카드, 현대카드, 삼성카드, BC카드

## 보안 원칙
- 금융사 로그인 ID/비밀번호 저장 안 함
- 금융사 자동 로그인/스크래핑 미구현
- 관리자 비밀번호 해시 저장
- 세션 쿠키 기반 인증
- 로그에는 민감 원문 대신 안전 메타데이터만 기록

## 향후 계획
- 카드사별 파서 확장 (현대/삼성/BC)
- 가져오기 오류 행 상세 편집
- 텔레그램 `/today`, `/add`, `/import` 연동
- 텔레그램 기본 명령(`/today`, `/month`, `/add`, `/cards`) 처리용 웹훅 구조
- 백업/내보내기 UI 고도화
- 다중 사용자 초대/권한 모델 확장

## 텔레그램 웹훅(초기 구조)
- 엔드포인트: `POST /api/telegram/webhook`
- 지원 명령:
- `/today`: 오늘 지출/수입 요약
- `/month`: 이번 달 요약
- `/add 금액 가맹점 [메모]`: 지출 거래 빠른 등록
- `/cards`: 이번 달 카드별 지출 요약
- `/import`: 확정 대기 중인 가져오기 목록 조회
- `/ok [코드]`: 가져오기 저장 확정 (코드 생략 시 최신 항목)
- 응답 메시지는 모두 한국어로 반환됩니다.
