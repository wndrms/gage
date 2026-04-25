-- 기존에 raw 파일 값으로 저장된 카드명을 "발급사 (뒷4자리)" 형식으로 정규화합니다.
-- 예: "536648******9959" → "삼성카드 (9959)"
-- 예: "본인205*"         → "신한카드 (205*)"
-- 예: "현대카드 X3 BOOST (760*)" → "현대카드 (760*)"

UPDATE cards
SET card_name = issuer || ' (' ||
    -- 16자리 마스킹 패턴 (숫자6개+*6개+숫자4개): 뒷 4자리
    CASE
        WHEN card_name ~ '^[0-9]{6}\*{6}[0-9]{4}$'
            THEN right(card_name, 4)
        -- "(숫자*)" 패턴 이미 있으면 그대로
        WHEN card_name ~ '\([0-9*]+\)$'
            THEN regexp_replace(card_name, '.*\(([0-9*]+)\)$', '\1')
        -- "본인XXX*" 패턴: 숫자+* 추출
        WHEN card_name ~ '[0-9]+\*$'
            THEN regexp_replace(card_name, '^.*?([0-9]+\*)$', '\1')
        -- 기타: 마지막 4자리 숫자
        WHEN card_name ~ '[0-9]{4}'
            THEN regexp_replace(card_name, '^.*([0-9]{4})[^0-9]*$', '\1')
        ELSE card_name
    END || ')',
    updated_at = now()
WHERE
    issuer IS NOT NULL
    AND issuer != ''
    AND issuer != '미분류'
    -- 이미 "발급사 (XXX)" 형식인 경우 건너뜀
    AND card_name NOT LIKE issuer || ' (%)'
    -- 뒷자리를 뽑을 수 있는 경우만
    AND (
        card_name ~ '^[0-9]{6}\*{6}[0-9]{4}$'
        OR card_name ~ '\([0-9*]+\)$'
        OR card_name ~ '[0-9]+\*$'
        OR card_name ~ '[0-9]{4}'
    );
