use sqlx::PgPool;
use std::time::Duration;
use tracing::{info, error};

/// Démarre le service de nettoyage (TTL pruning)
pub async fn start_pruning_service(pool: PgPool) {
    info!("🧹 Service de nettoyage (TTL pruning) démarré");
    
    let mut interval = tokio::time::interval(Duration::from_secs(60)); // Toutes les 60 secondes
    
    loop {
        interval.tick().await;
        
        // 1. Nettoyage des nœuds hors ligne (pas de heartbeat depuis > 5 minutes)
        let node_res = sqlx::query(
            "DELETE FROM nodes WHERE last_heartbeat < CURRENT_TIMESTAMP - INTERVAL '5 minutes'"
        )
        .execute(&pool)
        .await;
        
        match node_res {
            Ok(res) => {
                let affected = res.rows_affected();
                if affected > 0 {
                    info!("🗑️ {} nœuds inactifs supprimés (TTL)", affected);
                }
            },
            Err(e) => error!("❌ Erreur pruning nœuds: {:?}", e),
        }

        // 2. Fermeture automatique des sessions orphelines
        let session_res = sqlx::query(
            "UPDATE peer_sessions SET is_active = FALSE, ended_at = CURRENT_TIMESTAMP \
             WHERE is_active = TRUE AND node_id NOT IN (SELECT id FROM nodes)"
        )
        .execute(&pool)
        .await;

        match session_res {
            Ok(res) => {
                let affected = res.rows_affected();
                if affected > 0 {
                    info!("📉 {} sessions orphelines fermées", affected);
                }
            },
            Err(e) => error!("❌ Erreur pruning sessions: {:?}", e),
        }
        
        // 3. Optionnel : Suppression des utilisateurs anonymes inactifs (zéro log)
        // On garde les utilisateurs ayant des crédits, mais on supprime ceux sans activité longue
        let user_res = sqlx::query(
            "DELETE FROM users WHERE ed25519_pubkey IS NOT NULL \
             AND last_active < CURRENT_TIMESTAMP - INTERVAL '24 hours' \
             AND credits = 50" // Seulement si ils n'ont pas gagné de crédits (50 = base)
        )
        .execute(&pool)
        .await;

        match user_res {
            Ok(res) => {
                let affected = res.rows_affected();
                if affected > 0 {
                    info!("👤 {} comptes anonymes éphémères supprimés", affected);
                }
            },
            Err(e) => error!("❌ Erreur pruning users: {:?}", e),
        }
    }
}
