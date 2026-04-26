use anyhow::Result;
use argon2::{
    Argon2, PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn seed_defaults(pool: &PgPool, admin_password: &str) -> Result<()> {
    let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM users")
        .fetch_one(pool)
        .await?;

    if exists > 0 {
        return Ok(());
    }

    let user_id = Uuid::new_v4();
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(admin_password.as_bytes(), &salt)?
        .to_string();

    let mut tx = pool.begin().await?;

    sqlx::query(
        "INSERT INTO users (id, email, display_name, password_hash, role) VALUES ($1, NULL, $2, $3, 'admin')",
    )
    .bind(user_id)
    .bind("관리자")
    .bind(password_hash)
    .execute(&mut *tx)
    .await?;

    let categories = vec![
        ("식비", "expense"),
        ("카페", "expense"),
        ("교통", "expense"),
        ("쇼핑", "expense"),
        ("생활", "expense"),
        ("고정비", "expense"),
        ("의료", "expense"),
        ("문화", "expense"),
        ("기타", "expense"),
        ("급여", "income"),
        ("이체", "transfer"),
    ];

    for (idx, (name, typ)) in categories.iter().enumerate() {
        sqlx::query(
            "INSERT INTO categories (id, user_id, name, type, sort_order) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(*name)
        .bind(*typ)
        .bind(idx as i32)
        .execute(&mut *tx)
        .await?;
    }

    let accounts = vec![
        ("신한은행 입출금", "bank", Some("신한은행")),
        ("현금", "cash", None),
        ("카드 미청구금", "credit_card_liability", None),
    ];

    for (name, typ, institution) in accounts {
        sqlx::query(
            "INSERT INTO accounts (id, user_id, name, type, institution, currency) VALUES ($1, $2, $3, $4, $5, 'KRW')",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(name)
        .bind(typ)
        .bind(institution)
        .execute(&mut *tx)
        .await?;
    }

    for issuer in ["신한카드", "현대카드", "삼성카드", "BC카드"] {
        sqlx::query(
            "INSERT INTO card_presets (id, issuer, card_name, aliases, monthly_requirement, rules, benefits) VALUES ($1, $2, $3, $4, NULL, '{}'::jsonb, '[]'::jsonb)",
        )
        .bind(Uuid::new_v4())
        .bind(issuer)
        .bind(format!("{} 기본", issuer))
        .bind(Vec::<String>::new())
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    tracing::info!("기본 사용자와 샘플 데이터 시드가 완료되었습니다");
    Ok(())
}
