//! Camada de banco de dados (SQLite) para o servidor de login Hades.
//! Tabelas: accounts, characters.

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{rand_core::OsRng, SaltString};
use sqlx::{SqlitePool, Row};
use thiserror::Error;

use crate::messages::CharData;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("Erro SQL: {0}")]
    Sql(#[from] sqlx::Error),
    #[error("Credenciais inválidas")]
    InvalidCredentials,
    #[error("Nome de personagem já existe")]
    DuplicateName,
    #[error("Slots de personagem cheios")]
    SlotsFull,
    #[error("Personagem não encontrado")]
    CharNotFound,
    #[error("Erro de hash de senha: {0}")]
    PasswordHash(String),
}

/// Conexão com o banco de dados SQLite.
pub struct Db {
    pool: SqlitePool,
}

impl Db {
    /// Abre (ou cria) o banco de dados SQLite.
    pub async fn open(url: &str) -> Result<Self, DbError> {
        let pool = SqlitePool::connect(url).await?;
        Ok(Self { pool })
    }

    /// Inicializa o esquema do banco de dados.
    pub async fn migrate(&self) -> Result<(), DbError> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS accounts (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                username     TEXT    NOT NULL UNIQUE,
                password_hash TEXT   NOT NULL,
                sex          INTEGER NOT NULL DEFAULT 0,
                user_level   INTEGER NOT NULL DEFAULT 0,
                created_at   INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                banned_until INTEGER NOT NULL DEFAULT 0
            )"
        ).execute(&self.pool).await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS characters (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                account_id   INTEGER NOT NULL REFERENCES accounts(id),
                slot         INTEGER NOT NULL DEFAULT 0,
                name         TEXT    NOT NULL UNIQUE,
                job          INTEGER NOT NULL DEFAULT 0,
                level        INTEGER NOT NULL DEFAULT 1,
                job_level    INTEGER NOT NULL DEFAULT 1,
                exp          INTEGER NOT NULL DEFAULT 0,
                job_exp      INTEGER NOT NULL DEFAULT 0,
                zeny         INTEGER NOT NULL DEFAULT 0,
                hp           INTEGER NOT NULL DEFAULT 40,
                max_hp       INTEGER NOT NULL DEFAULT 40,
                sp           INTEGER NOT NULL DEFAULT 11,
                max_sp       INTEGER NOT NULL DEFAULT 11,
                str_stat     INTEGER NOT NULL DEFAULT 1,
                agi_stat     INTEGER NOT NULL DEFAULT 1,
                vit_stat     INTEGER NOT NULL DEFAULT 1,
                int_stat     INTEGER NOT NULL DEFAULT 1,
                dex_stat     INTEGER NOT NULL DEFAULT 1,
                luk_stat     INTEGER NOT NULL DEFAULT 1,
                skill_points INTEGER NOT NULL DEFAULT 0,
                job_points   INTEGER NOT NULL DEFAULT 0,
                hair_style   INTEGER NOT NULL DEFAULT 0,
                hair_color   INTEGER NOT NULL DEFAULT 0,
                sex          INTEGER NOT NULL DEFAULT 0,
                map          TEXT    NOT NULL DEFAULT 'prontera',
                pos_x        INTEGER NOT NULL DEFAULT 156,
                pos_y        INTEGER NOT NULL DEFAULT 144,
                created_at   INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
            )"
        ).execute(&self.pool).await?;

        Ok(())
    }

    /// Cria uma conta com senha hasheada com Argon2id.
    pub async fn create_account(&self, username: &str, password: &str, sex: u8) -> Result<u32, DbError> {
        let salt   = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash   = argon2.hash_password(password.as_bytes(), &salt)
            .map_err(|e| DbError::PasswordHash(e.to_string()))?
            .to_string();

        let row = sqlx::query(
            "INSERT INTO accounts (username, password_hash, sex) VALUES (?, ?, ?) RETURNING id"
        )
        .bind(username)
        .bind(&hash)
        .bind(sex)
        .fetch_one(&self.pool).await?;

        Ok(row.get::<i64, _>(0) as u32)
    }

    /// Autentica: verifica a senha SHA-256 do client contra o hash Argon2 do DB.
    /// O client envia SHA-256(senha_texto) — o server armazena Argon2(senha_texto).
    /// Como não temos a senha em texto aqui, aceitamos o SHA-256 como proxy.
    /// TODO: implementar SRP-6a para autenticação sem enviar hash.
    pub async fn authenticate(&self, username: &str, password_sha256_hex: &str) -> Result<(u32, u8, u32), DbError> {
        let row = sqlx::query(
            "SELECT id, password_hash, sex, user_level, banned_until FROM accounts WHERE username = ?"
        )
        .bind(username)
        .fetch_optional(&self.pool).await?;

        let row = row.ok_or(DbError::InvalidCredentials)?;
        let id:           u32 = row.get::<i64, _>(0) as u32;
        let stored_hash:  String = row.get(1);
        let sex:          u8  = row.get::<i64, _>(2) as u8;
        let user_level:   u32 = row.get::<i64, _>(3) as u32;
        let banned_until: i64 = row.get(4);

        if banned_until > chrono_now() {
            return Err(DbError::InvalidCredentials); // TODO: erro de ban específico
        }

        let is_valid = if stored_hash.starts_with('$') {
            match PasswordHash::new(&stored_hash) {
                Ok(parsed) => Argon2::default().verify_password(password_sha256_hex.as_bytes(), &parsed).is_ok(),
                Err(_) => false,
            }
        } else {
            stored_hash == password_sha256_hex
        };

        if !is_valid {
            return Err(DbError::InvalidCredentials);
        }

        Ok((id, sex, user_level))
    }

    /// Cria uma conta simples com SHA-256 como hash (modo dev).
    pub async fn create_account_dev(&self, username: &str, password_sha256: &str, sex: u8) -> Result<u32, DbError> {
        let row = sqlx::query(
            "INSERT OR IGNORE INTO accounts (username, password_hash, sex) VALUES (?, ?, ?) RETURNING id"
        )
        .bind(username)
        .bind(password_sha256)
        .bind(sex)
        .fetch_optional(&self.pool).await?;

        match row {
            Some(r) => Ok(r.get::<i64, _>(0) as u32),
            None => {
                let r = sqlx::query("SELECT id FROM accounts WHERE username = ?")
                    .bind(username).fetch_one(&self.pool).await?;
                Ok(r.get::<i64, _>(0) as u32)
            }
        }
    }

    /// Lista os personagens de uma conta.
    pub async fn list_chars(&self, account_id: u32) -> Result<Vec<CharData>, DbError> {
        let rows = sqlx::query(
            "SELECT id, slot, name, job, level, job_level, exp, job_exp, zeny,
                    hp, max_hp, sp, max_sp, str_stat, agi_stat, vit_stat,
                    int_stat, dex_stat, luk_stat, skill_points, job_points,
                    hair_style, hair_color, sex, map, pos_x, pos_y
             FROM characters WHERE account_id = ? ORDER BY slot ASC"
        )
        .bind(account_id)
        .fetch_all(&self.pool).await?;

        Ok(rows.iter().map(|r| CharData {
            gid:          r.get::<i64, _>(0) as u32,
            slot:         r.get::<i64, _>(1) as u8,
            name:         r.get(2),
            job:          r.get::<i64, _>(3) as u16,
            level:        r.get::<i64, _>(4) as u16,
            job_level:    r.get::<i64, _>(5) as u16,
            exp:          r.get::<i64, _>(6) as u64,
            job_exp:      r.get::<i64, _>(7) as u64,
            zeny:         r.get::<i64, _>(8) as u32,
            hp:           r.get::<i64, _>(9) as u32,
            max_hp:       r.get::<i64, _>(10) as u32,
            sp:           r.get::<i64, _>(11) as u32,
            max_sp:       r.get::<i64, _>(12) as u32,
            str:          r.get::<i64, _>(13) as u8,
            agi:          r.get::<i64, _>(14) as u8,
            vit:          r.get::<i64, _>(15) as u8,
            int_stat:     r.get::<i64, _>(16) as u8,
            dex:          r.get::<i64, _>(17) as u8,
            luk:          r.get::<i64, _>(18) as u8,
            skill_points: r.get::<i64, _>(19) as u16,
            job_points:   r.get::<i64, _>(20) as u16,
            hair_style:   r.get::<i64, _>(21) as u16,
            hair_color:   r.get::<i64, _>(22) as u16,
            sex:          r.get::<i64, _>(23) as u8,
            map:          r.get(24),
            pos_x:        r.get::<i64, _>(25) as u16,
            pos_y:        r.get::<i64, _>(26) as u16,
            // campos visuais extras (zerados por ora)
            body: 0, weapon: 0, shield: 0,
            accessory: 0, accessory2: 0, accessory3: 0,
            body_palette: 0, speed: 150,
        }).collect())
    }

    /// Cria um personagem.
    pub async fn create_char(
        &self,
        account_id: u32,
        name: &str,
        str: u8, agi: u8, vit: u8, int: u8, dex: u8, luk: u8,
        hair_style: u16, hair_color: u16, sex: u8,
    ) -> Result<CharData, DbError> {
        // Valida soma dos stats (deve ser 6)
        let sum = str as u16 + agi as u16 + vit as u16 + int as u16 + dex as u16 + luk as u16;
        if sum != 6 {
            return Err(DbError::SlotsFull); // TODO: erro específico de stats inválidos
        }

        // Próximo slot disponível
        let slot_row = sqlx::query(
            "SELECT COUNT(*) FROM characters WHERE account_id = ?"
        ).bind(account_id).fetch_one(&self.pool).await?;
        let slot: i64 = slot_row.get(0);
        if slot >= 9 { return Err(DbError::SlotsFull); }

        let result = sqlx::query(
            "INSERT INTO characters
                (account_id, slot, name, str_stat, agi_stat, vit_stat, int_stat, dex_stat,
                 luk_stat, hair_style, hair_color, sex)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             RETURNING id"
        )
        .bind(account_id).bind(slot as u8).bind(name)
        .bind(str).bind(agi).bind(vit).bind(int).bind(dex).bind(luk)
        .bind(hair_style).bind(hair_color).bind(sex)
        .fetch_optional(&self.pool).await;

        match result {
            Ok(Some(row)) => {
                let gid = row.get::<i64, _>(0) as u32;
                Ok(CharData::new_novice(gid, name.to_string(), str, agi, vit, int, dex, luk, hair_style, hair_color, sex, slot as u8))
            }
            Ok(None) => Err(DbError::DuplicateName),
            Err(e) if e.to_string().contains("UNIQUE") => Err(DbError::DuplicateName),
            Err(e) => Err(DbError::Sql(e)),
        }
    }

    /// Deleta um personagem (apenas se pertence à conta).
    pub async fn delete_char(&self, account_id: u32, gid: u32) -> Result<(), DbError> {
        let rows = sqlx::query(
            "DELETE FROM characters WHERE id = ? AND account_id = ?"
        )
        .bind(gid).bind(account_id)
        .execute(&self.pool).await?;

        if rows.rows_affected() == 0 {
            Err(DbError::CharNotFound)
        } else {
            Ok(())
        }
    }

    /// Obtém um personagem pelo GID ou pelo slot (0..8).
    pub async fn get_char(&self, account_id: u32, gid_or_slot: u32) -> Result<CharData, DbError> {
        let chars = self.list_chars(account_id).await?;
        chars
            .into_iter()
            .find(|c| c.gid == gid_or_slot || c.slot as u32 == gid_or_slot)
            .ok_or(DbError::CharNotFound)
    }
}

fn chrono_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_db_flow() {
        let db = Db::open("sqlite::memory:").await.unwrap();
        db.migrate().await.unwrap();

        // 1. Criar conta dev
        let aid = db.create_account_dev("testuser", "secret_hash", 0).await.unwrap();
        assert!(aid > 0);

        // 2. Autenticar
        let (auth_aid, sex, user_level) = db.authenticate("testuser", "secret_hash").await.unwrap();
        assert_eq!(auth_aid, aid);
        assert_eq!(sex, 0);
        assert_eq!(user_level, 0);

        // Falha com hash errado
        assert!(matches!(
            db.authenticate("testuser", "wrong_hash").await,
            Err(DbError::InvalidCredentials)
        ));

        // 3. Criar personagem (stats somando 6)
        let ch = db.create_char(aid, "NoviceHero", 1, 1, 1, 1, 1, 1, 1, 0, 0).await.unwrap();
        assert_eq!(ch.name, "NoviceHero");
        assert_eq!(ch.slot, 0);
        assert_eq!(ch.level, 1);

        // 4. Listar personagens
        let list = db.list_chars(aid).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].gid, ch.gid);
        assert_eq!(list[0].body_palette, 0);

        // 5. Obter personagem
        let fetched = db.get_char(aid, ch.gid).await.unwrap();
        assert_eq!(fetched.name, "NoviceHero");

        // 6. Deletar personagem
        db.delete_char(aid, ch.gid).await.unwrap();
        let list_after = db.list_chars(aid).await.unwrap();
        assert!(list_after.is_empty());
    }
}

