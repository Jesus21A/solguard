use anchor_lang::prelude::*;

// Reemplaza este ID con la pubkey de programs/solguard-keypair.json
declare_id!("9cuFsdRhYpm2JjJ1NbzA8W2nG5tSgzHto3ABt5qycsN9");

#[program]
pub mod solguard {
    use super::*;

    /// Registra KYC y emite un SBT (Soulbound Token) para la wallet.
    pub fn submit_kyc(
        ctx: Context<SubmitKyc>,
        kyc_level: u8,
        nombre_hash: [u8; 16],
        doc_hash: [u8; 16],
    ) -> Result<()> {
        let record = &mut ctx.accounts.wallet_record;
        // init_if_needed inicializa kyc_level a 0 — solo bloqueamos si ya fue verificado
        if record.kyc_level > 0 {
            return err!(SolGuardError::AlreadyVerified);
        }

        let clock = Clock::get()?;
        record.wallet = ctx.accounts.wallet.key();
        record.kyc_level = kyc_level;
        record.risk_score = 1;
        record.frozen = false;
        record.kyc_at = clock.unix_timestamp;
        record.created_at = clock.unix_timestamp;
        record.nombre_hash = nombre_hash;
        record.doc_hash = doc_hash;

        emit!(KycMinted {
            wallet: ctx.accounts.wallet.key(),
            kyc_level,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Actualiza el risk_score on-chain (llamado por el Oracle Signer).
    pub fn update_risk_score(
        ctx: Context<UpdateRiskScore>,
        risk_score: u8,
        frozen: Option<bool>,
    ) -> Result<()> {
        require!(
            risk_score >= 1 && risk_score <= 10,
            SolGuardError::InvalidRiskScore
        );

        let record = &mut ctx.accounts.wallet_record;
        record.risk_score = risk_score;
        record.frozen = frozen.unwrap_or(risk_score >= 7);

        let clock = Clock::get()?;
        emit!(RiskScoreUpdated {
            wallet: ctx.accounts.wallet.key(),
            risk_score,
            frozen: record.frozen,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Valida acceso — cualquier juego puede hacer CPI a esta instrucción.
    pub fn validate_access(ctx: Context<ValidateAccess>) -> Result<()> {
        let record = &ctx.accounts.wallet_record;
        require!(record.kyc_level > 0, SolGuardError::NoKyc);
        require!(record.risk_score < 7, SolGuardError::HighRisk);
        require!(!record.frozen, SolGuardError::WalletFrozen);
        Ok(())
    }
}

// ── Contexts ──────────────────────────────────────────────────────────────────

#[derive(Accounts)]
pub struct SubmitKyc<'info> {
    #[account(
        init_if_needed,
        payer = authority,
        space = 8 + WalletRecord::LEN,
        seeds = [b"wallet_record", wallet.key().as_ref()],
        bump
    )]
    pub wallet_record: Account<'info, WalletRecord>,
    /// CHECK: Wallet del usuario — recibe el SBT sin necesidad de firmar
    pub wallet: UncheckedAccount<'info>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateRiskScore<'info> {
    #[account(
        mut,
        seeds = [b"wallet_record", wallet.key().as_ref()],
        bump,
    )]
    pub wallet_record: Account<'info, WalletRecord>,
    /// CHECK: Wallet del usuario
    pub wallet: UncheckedAccount<'info>,
    pub oracle: Signer<'info>,
}

#[derive(Accounts)]
pub struct ValidateAccess<'info> {
    #[account(
        seeds = [b"wallet_record", wallet.key().as_ref()],
        bump,
    )]
    pub wallet_record: Account<'info, WalletRecord>,
    /// CHECK: Wallet del usuario
    pub wallet: UncheckedAccount<'info>,
}

// ── State ─────────────────────────────────────────────────────────────────────

#[account]
pub struct WalletRecord {
    pub wallet: Pubkey,        // 32
    pub kyc_level: u8,         //  1
    pub risk_score: u8,        //  1
    pub frozen: bool,          //  1
    pub kyc_at: i64,           //  8
    pub created_at: i64,       //  8
    pub nombre_hash: [u8; 16], // 16
    pub doc_hash: [u8; 16],    // 16
}

impl WalletRecord {
    pub const LEN: usize = 32 + 1 + 1 + 1 + 8 + 8 + 16 + 16; // 83
}

// ── Events ────────────────────────────────────────────────────────────────────

#[event]
pub struct KycMinted {
    pub wallet: Pubkey,
    pub kyc_level: u8,
    pub timestamp: i64,
}

#[event]
pub struct RiskScoreUpdated {
    pub wallet: Pubkey,
    pub risk_score: u8,
    pub frozen: bool,
    pub timestamp: i64,
}

// ── Errors ────────────────────────────────────────────────────────────────────

#[error_code]
pub enum SolGuardError {
    #[msg("Esta wallet ya tiene KYC verificado")]
    AlreadyVerified,
    #[msg("risk_score debe estar entre 1 y 10")]
    InvalidRiskScore,
    #[msg("Wallet sin KYC — registra primero")]
    NoKyc,
    #[msg("Risk score demasiado alto — acceso denegado")]
    HighRisk,
    #[msg("Wallet congelada — contacta a soporte")]
    WalletFrozen,
}
