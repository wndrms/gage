-- 카드 혜택 프리셋 시드 데이터
-- 신한카드 Deep On Platinum+, 삼성카드 iD ON, 현대카드 X3 BOOST, 현대카드 무신사

INSERT INTO card_presets (id, issuer, card_name, aliases, monthly_requirement, rules, benefits, parse_text)
VALUES

-- ─── 신한카드 Deep On Platinum+ ───────────────────────────────────────────────
(
  '11111111-0001-0001-0001-000000000001',
  '신한카드',
  'Deep On Platinum+',
  ARRAY['신한 Deep On', 'Deep On Platinum', '딥온 플래티넘'],
  300000,
  '{"excluded":[{"merchant_contains":"상품권"},{"merchant_contains":"선불카드"},{"merchant_contains":"충전"}]}'::jsonb,
  '[
    {
      "name": "간편결제 온라인 10% 할인",
      "match": {"merchant_keywords": ["SOL페이","삼성페이","LG페이","스마일페이","네이버페이","카카오페이","페이코","SSG페이","L페이","SK페이"]},
      "discount": {"type": "percent", "value": 10, "monthly_cap": 40000, "per_tx_cap": 5000},
      "min_amount": 20000
    },
    {
      "name": "온라인 배달/슈퍼 추가 10% 할인",
      "match": {"merchant_keywords": ["요기요","GS프레시","오아시스"]},
      "discount": {"type": "percent", "value": 10, "monthly_cap": 40000, "per_tx_cap": 10000},
      "min_amount": 20000
    },
    {
      "name": "편의점 20% 할인",
      "match": {"merchant_keywords": ["GS25","CU","세븐일레븐","이마트24"]},
      "discount": {"type": "percent", "value": 20, "monthly_cap": 30000, "per_tx_cap": 10000},
      "min_amount": 10000
    },
    {
      "name": "올리브영/다이소 20% 할인",
      "match": {"merchant_keywords": ["올리브영","다이소"]},
      "discount": {"type": "percent", "value": 20, "monthly_cap": 30000, "per_tx_cap": 10000},
      "min_amount": 10000
    },
    {
      "name": "커피 20% 할인",
      "match": {"merchant_keywords": ["스타벅스","투썸플레이스"]},
      "discount": {"type": "percent", "value": 20, "monthly_cap": 30000, "per_tx_cap": 10000},
      "min_amount": 10000
    },
    {
      "name": "월정기구독 20% 할인",
      "match": {"merchant_keywords": ["쿠팡","위메프","리디북스","프레딧"]},
      "discount": {"type": "percent", "value": 20, "monthly_cap": 30000, "per_tx_cap": 10000},
      "min_amount": 10000
    },
    {
      "name": "제주항공/에어부산 10% 할인",
      "match": {"merchant_keywords": ["제주항공","에어부산"]},
      "discount": {"type": "percent", "value": 10, "monthly_cap": 30000, "per_tx_cap": 30000},
      "min_amount": 10000
    }
  ]'::jsonb,
  '신한카드 Deep On Platinum+ 혜택 텍스트 (2026-04-27 파싱)'
),

-- ─── 삼성카드 iD ON ───────────────────────────────────────────────────────────
(
  '11111111-0002-0002-0002-000000000002',
  '삼성카드',
  'iD ON',
  ARRAY['삼성 iD ON', '삼성카드 iD ON'],
  300000,
  '{"excluded":[{"merchant_contains":"상품권"},{"merchant_contains":"선불카드"},{"merchant_contains":"충전"},{"merchant_contains":"무이자할부"}]}'::jsonb,
  '[
    {
      "name": "온라인 간편결제 5% 할인",
      "match": {"merchant_keywords": ["삼성페이","네이버페이","카카오페이","PAYCO","스마일페이","SSG페이","쿠페이","SK페이"]},
      "discount": {"type": "percent", "value": 5, "monthly_cap": 20000, "per_tx_cap": 5000},
      "min_amount": 1000
    },
    {
      "name": "스트리밍 50% 할인",
      "match": {"merchant_keywords": ["넷플릭스","웨이브","티빙","왓챠","멜론","FLO"]},
      "discount": {"type": "percent", "value": 50, "monthly_cap": 10000, "per_tx_cap": 5000},
      "min_amount": 1000
    },
    {
      "name": "편의점/헬스뷰티/생활잡화 10% 할인",
      "match": {"merchant_keywords": ["CU","GS25","세븐일레븐","미니스톱","이마트24","올리브영","롭스","랄라블라","다이소"]},
      "discount": {"type": "percent", "value": 10, "monthly_cap": 5000, "per_tx_cap": 5000},
      "min_amount": 1000
    },
    {
      "name": "해외 1.5% 할인",
      "discount": {"type": "percent", "value": 2, "monthly_cap": 500000},
      "min_amount": 1000
    }
  ]'::jsonb,
  '삼성카드 iD ON 혜택 텍스트 (2026-04-27 파싱)'
),

-- ─── 현대카드 X3 BOOST ────────────────────────────────────────────────────────
(
  '11111111-0003-0003-0003-000000000003',
  '현대카드',
  'X3 BOOST',
  ARRAY['현대카드 X3', 'X3BOOST', 'X3 부스트'],
  500000,
  '{"excluded":[{"merchant_contains":"상품권"},{"merchant_contains":"선불카드"},{"merchant_contains":"공과금"},{"merchant_contains":"등록금"},{"merchant_contains":"관리비"}]}'::jsonb,
  '[
    {
      "name": "기본 1.5% 할인 (당월 100만원 이상)",
      "discount": {"type": "percent", "value": 2, "monthly_cap": 100000},
      "min_amount": 1000
    },
    {
      "name": "온라인 간편결제 5% 할인",
      "match": {"merchant_keywords": ["Apple Pay","삼성페이","네이버페이","카카오페이","스마일페이","SSG페이","쿠페이","SK페이"]},
      "discount": {"type": "percent", "value": 5, "monthly_cap": 10000, "per_tx_cap": 5000},
      "min_amount": 1000
    },
    {
      "name": "해외 가맹점 5% 할인",
      "discount": {"type": "percent", "value": 5, "monthly_cap": 10000},
      "min_amount": 1000
    }
  ]'::jsonb,
  '현대카드 X3 BOOST 혜택 텍스트 (2026-04-27 파싱)'
),

-- ─── 현대카드 무신사 ──────────────────────────────────────────────────────────
(
  '11111111-0004-0004-0004-000000000004',
  '현대카드',
  '무신사카드',
  ARRAY['현대카드 무신사', 'MUSINSA카드'],
  300000,
  '{"excluded":[{"merchant_contains":"상품권"},{"merchant_contains":"선불카드"}]}'::jsonb,
  '[
    {
      "name": "무신사/솔드아웃 5% 할인",
      "match": {"merchant_keywords": ["무신사","솔드아웃","MUSINSA","SOLDOUT"]},
      "discount": {"type": "percent", "value": 5, "monthly_cap": 30000, "per_tx_cap": 30000},
      "min_amount": 1000
    }
  ]'::jsonb,
  '현대카드 무신사카드 혜택 텍스트 (2026-04-27 파싱)'
)

ON CONFLICT (id) DO NOTHING;
