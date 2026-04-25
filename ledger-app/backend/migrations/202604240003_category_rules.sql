CREATE TABLE IF NOT EXISTS merchant_category_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    keyword TEXT NOT NULL,
    keyword_normalized TEXT NOT NULL,
    category_id UUID NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    priority INTEGER NOT NULL DEFAULT 100,
    source TEXT NOT NULL DEFAULT 'user' CHECK (source IN ('user', 'learned', 'seed')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, keyword_normalized)
);

CREATE INDEX IF NOT EXISTS idx_mcr_user_priority
    ON merchant_category_rules (user_id, priority DESC);

-- 기존 유저가 있으면 시드 규칙 삽입 (카테고리가 존재할 때만)
DO $$
DECLARE
    v_user_id UUID;
BEGIN
    SELECT id INTO v_user_id FROM users ORDER BY created_at ASC LIMIT 1;
    IF v_user_id IS NULL THEN
        RETURN;
    END IF;

    INSERT INTO merchant_category_rules (user_id, keyword, keyword_normalized, category_id, priority, source)
    SELECT v_user_id,
           kw.keyword,
           lower(regexp_replace(kw.keyword, '\s+', '', 'g')),
           c.id,
           kw.priority,
           'seed'
    FROM (VALUES
        -- 식비
        ('맥도날드',         '식비', 220),
        ('버거킹',           '식비', 220),
        ('롯데리아',         '식비', 220),
        ('맘스터치',         '식비', 220),
        ('서브웨이',         '식비', 220),
        ('도미노피자',       '식비', 220),
        ('피자헛',           '식비', 220),
        ('배달의민족',       '식비', 200),
        ('요기요',           '식비', 200),
        ('쿠팡이츠',         '식비', 200),
        ('GS25',             '식비', 180),
        ('세븐일레븐',       '식비', 180),
        ('이마트24',         '식비', 180),
        ('이마트',           '식비', 170),
        ('홈플러스',         '식비', 170),
        ('롯데마트',         '식비', 170),
        ('코스트코',         '식비', 170),
        ('농협하나로',       '식비', 170),
        -- 카페
        ('스타벅스',         '카페', 230),
        ('투썸플레이스',     '카페', 230),
        ('이디야',           '카페', 230),
        ('커피빈',           '카페', 230),
        ('메가커피',         '카페', 230),
        ('빽다방',           '카페', 230),
        ('컴포즈커피',       '카페', 230),
        ('할리스',           '카페', 230),
        ('파스쿠찌',         '카페', 230),
        ('엔제리너스',       '카페', 230),
        ('폴바셋',           '카페', 230),
        -- 교통
        ('카카오택시',       '교통', 220),
        ('카카오T',          '교통', 220),
        ('우티',             '교통', 220),
        ('티머니',           '교통', 220),
        ('한국도로공사',     '교통', 220),
        ('고속도로',         '교통', 210),
        ('SK주유소',         '교통', 200),
        ('GS칼텍스',         '교통', 200),
        ('에쓰오일',         '교통', 200),
        ('현대오일뱅크',     '교통', 200),
        ('주유소',           '교통', 180),
        -- 쇼핑
        ('쿠팡',             '쇼핑', 200),
        ('11번가',           '쇼핑', 200),
        ('G마켓',            '쇼핑', 200),
        ('옥션',             '쇼핑', 200),
        ('위메프',           '쇼핑', 200),
        ('티몬',             '쇼핑', 200),
        ('올리브영',         '쇼핑', 200),
        ('다이소',           '쇼핑', 200),
        ('무신사',           '쇼핑', 200),
        ('크림',             '쇼핑', 200),
        ('에이블리',         '쇼핑', 200),
        ('지그재그',         '쇼핑', 200),
        ('오늘의집',         '쇼핑', 200),
        ('마켓컬리',         '쇼핑', 200),
        ('SSG',              '쇼핑', 200),
        ('네이버페이',       '쇼핑', 150),
        ('토스페이',         '쇼핑', 150),
        ('카카오페이',       '쇼핑', 150),
        ('KCP',              '쇼핑', 130),
        -- 생활
        ('당근마켓',         '생활', 180),
        ('번개장터',         '생활', 180),
        ('이케아',           '생활', 200),
        ('홈디포',           '생활', 180),
        ('한샘',             '생활', 180),
        ('청소',             '생활', 170),
        ('세탁',             '생활', 170),
        ('편의점',           '생활', 150),
        ('CU',               '생활', 180),
        -- 고정비
        ('SK텔레콤',         '고정비', 230),
        ('KT',               '고정비', 220),
        ('LG U+',            '고정비', 230),
        ('넷플릭스',         '고정비', 230),
        ('유튜브프리미엄',   '고정비', 230),
        ('디즈니플러스',     '고정비', 230),
        ('웨이브',           '고정비', 230),
        ('티빙',             '고정비', 230),
        ('왓챠',             '고정비', 230),
        ('스포티파이',       '고정비', 230),
        ('애플',             '고정비', 180),
        ('구글',             '고정비', 150),
        ('한국전력',         '고정비', 230),
        ('도시가스',         '고정비', 230),
        ('네이버플러스',     '고정비', 230),
        ('네이버클라우드',   '고정비', 220),
        ('비바리퍼블리카',   '고정비', 200),
        -- 의료
        ('병원',             '의료', 180),
        ('치과',             '의료', 200),
        ('약국',             '의료', 200),
        ('한의원',           '의료', 200),
        ('피부과',           '의료', 200),
        ('안과',             '의료', 200),
        -- 문화
        ('CGV',              '문화', 230),
        ('메가박스',         '문화', 230),
        ('롯데시네마',       '문화', 230),
        ('교보문고',         '문화', 220),
        ('YES24',            '문화', 220),
        ('알라딘',           '문화', 220),
        ('인터파크',         '문화', 200),
        ('인프런',           '문화', 220),
        ('패스트캠퍼스',     '문화', 220),
        ('클래스101',        '문화', 220),
        ('짐',               '문화', 150),
        ('헬스',             '문화', 150),
        ('요가',             '문화', 170),
        ('필라테스',         '문화', 200)
    ) AS kw(keyword, category_name, priority)
    JOIN categories c
      ON c.user_id = v_user_id
     AND c.name = kw.category_name
     AND c.type = 'expense'
    ON CONFLICT (user_id, keyword_normalized) DO NOTHING;
END $$;
