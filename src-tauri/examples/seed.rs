//! Popula um banco **vazio** com dados fictícios de demonstração.
//!
//! ```text
//! BOREAL_SEED_ADMIN_PASSWORD='...' cargo run --example seed --features dev-seed -- --db <caminho/boreal.db>
//! ```
//!
//! - `--db` é obrigatório: o seed nunca adivinha qual banco usar. O aplicativo grava o dele em
//!   `%APPDATA%\br.com.boreal.pms\boreal.db` (Windows); a chave vem do cofre de credenciais do sistema.
//! - `--user <nome>` (padrão `admin`) é o administrador usado; `BOREAL_SEED_ADMIN_PASSWORD` é a senha dele (o usuário é criado se ainda não houver usuários).
//! - `BOREAL_SEED_RECEPCAO_PASSWORD` (opcional) cria o usuário `recepcao` com essa senha temporária.

use std::path::PathBuf;
use std::process::ExitCode;

use boreal_pms_lib::app::initialize;
use boreal_pms_lib::db::keystore::OsKeyStore;
use boreal_pms_lib::seed;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut db: Option<PathBuf> = None;
    let mut user = String::from("admin");
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--db" => db = args.next().map(PathBuf::from),
            "--user" => user = args.next().unwrap_or(user),
            other => {
                eprintln!("Argumento desconhecido: {other}");
                return ExitCode::from(2);
            }
        }
    }
    let Some(db) = db else {
        eprintln!("Informe o banco com --db <caminho/boreal.db>.");
        return ExitCode::from(2);
    };
    let Ok(admin_password) = std::env::var("BOREAL_SEED_ADMIN_PASSWORD") else {
        eprintln!("Defina BOREAL_SEED_ADMIN_PASSWORD com a senha do administrador.");
        return ExitCode::from(2);
    };
    let recepcao_password = std::env::var("BOREAL_SEED_RECEPCAO_PASSWORD").ok();

    let app = match initialize(&db, &OsKeyStore::new()) {
        Ok(app) => app,
        Err(e) => {
            eprintln!("Não foi possível abrir o banco: {e}");
            return ExitCode::FAILURE;
        }
    };
    match seed::run(&app, &user, &admin_password, recepcao_password.as_deref()) {
        Ok(s) => {
            println!(
                "Seed concluído: {} quartos, {} hóspedes, {} reservas, {} hospedagens, {} bloqueio(s) manual(is), {} ordens de manutenção.",
                s.rooms, s.guests, s.reservations, s.stays, s.blocks, s.maintenance_orders
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Seed não executado: {e}");
            ExitCode::FAILURE
        }
    }
}
