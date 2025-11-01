#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use alloc::vec::Vec;
use stylus_sdk::{
    alloy_primitives::{Address, Bytes, U256},
    prelude::*,
    storage::StorageMap, 
    block, 
    msg,   
};

/// 5 años en segundos (5 * 365 * 24 * 60 * 60)
/// Usamos U256 para la resta
const FIVE_YEARS_IN_SECONDS: U256 = U256::from_limbs([157_680_000, 0, 0, 0]);

// --- 1. Definición del Registro de Préstamo ---
// Esta es la nueva "plantilla" para cada préstamo.
#[derive(Default, Debug, EthAbiType, EthAbiCodec, Clone)]
pub struct LoanRecord {
    /// El banco que reporta el préstamo
    provider: Address,
    /// Cuándo se OTORGÓ el préstamo (para el filtro de 5 años)
    timestamp_issued: U256,
    /// Monto del préstamo (puede ser útil para el analista)
    loan_amount: U256,
    /// 'true' si el préstamo ya fue pagado
    is_paid: bool,
    /// Cuándo se CONSUMÓ el pago
    timestamp_paid: U256, // Será 0 si 'is_paid' es 'false'
}

// --- 2. Almacenamiento del Contrato ---
#[sol_storage]
#[entrypoint]
pub struct LoanComplianceLedger {
    /// BASE DE DATOS: Address (usuario) => Lista [Vec] de sus préstamos
    user_loans: StorageMap<Address, Vec<LoanRecord>>,
}

// --- 3. Lógica del Contrato ---
#[external]
impl LoanComplianceLedger {
    
    /// --- CONSTRUCTOR ---
    pub fn new() -> Result<Self, Vec<u8>> {
        Ok(Self::default())
    }

    /// --- FUNCIÓN DE ESCRITURA 1: REGISTRAR UN NUEVO PRÉSTAMO ---
    /// Un banco llama a esto para registrar un préstamo que acaba de otorgar.
    pub fn add_loan_record(
        &mut self,
        user_address: Address, // La wallet del cliente
        loan_amount: U256,     // El monto que se le prestó
    ) -> Result<(), Vec<u8>> {
        
        // Quien llama es el banco (msg.sender)
        let provider_address = msg::sender(); 

        let new_loan = LoanRecord {
            provider: provider_address, 
            timestamp_issued: block::timestamp(), // Se registra cuándo se OTORGÓ
            loan_amount,
            is_paid: false, // El préstamo inicia como NO pagado
            timestamp_paid: U256::ZERO, // Aún no hay fecha de pago
        };

        let mut loan_list = self.user_loans.get(user_address);
        loan_list.push(new_loan);
        self.user_loans.insert(user_address, loan_list);

        Ok(())
    }

    /// --- FUNCIÓN DE ESCRITURA 2: MARCAR UN PRÉSTAMO COMO PAGADO ---
    /// El banco llama a esto cuando el cliente consuma el pago.
    pub fn mark_loan_as_paid(
        &mut self,
        user_address: Address, // La wallet del cliente
        loan_index: U256,      // El índice del préstamo en la lista
    ) -> Result<(), Vec<u8>> {
        
        let bank_address = msg::sender();

        // Obtenemos la lista de préstamos de forma mutable
        let mut loan_list = self.user_loans.get_mut(user_address);
        
        // Convertimos el U256 a usize para usarlo como índice
        let index = loan_index.to::<usize>();

        // Verificamos que el índice exista en la lista
        if let Some(loan) = loan_list.get_mut(index) {
            
            // ¡GUARDIA DE SEGURIDAD!
            // Solo el banco que OTORGÓ el préstamo puede marcarlo como pagado.
            if loan.provider != bank_address {
                return Err(b"NOT_ORIGINAL_PROVIDER".to_vec());
            }

            // Verificamos que no esté ya pagado
            if loan.is_paid {
                return Err(b"LOAN_ALREADY_PAID".to_vec());
            }

            // Actualizamos el registro
            loan.is_paid = true;
            loan.timestamp_paid = block::timestamp(); // Esta es la "fecha de consumación"

            // Guardamos la lista modificada
            loan_list.save();
            Ok(())

        } else {
            // Si el índice no existe
            Err(b"LOAN_INDEX_OUT_OF_BOUNDS".to_RECT_vec())
        }
    }

    /// --- FUNCIÓN DE LECTURA 1: OBTENER HISTORIAL BRUTO ---
    /// Devuelve la lista completa de préstamos de un usuario.
    #[view]
    pub fn get_loan_history(&self, user_address: Address) -> Result<Vec<LoanRecord>, Vec<u8>> {
        Ok(self.user_loans.get(user_address))
    }

    /// --- FUNCIÓN DE LECTURA 2: OBTENER PORCENTAJE DE CUMPLIMIENTO (ÚLTIMOS 5 AÑOS) ---
    /// Esta es la función que llamaría el prestamista para analizar.
    #[view]
    pub fn get_compliance_percentage(&self, user_address: Address) -> Result<U256, Vec<u8>> {
        
        let mut total_loans_in_period = U256::ZERO;
        let mut paid_loans_in_period = U256::ZERO;

        // 1. Calcular el punto de corte (timestamp de hace 5 años)
        let now = block::timestamp();
        // Usamos saturating_sub para evitar underflow si la blockchain es muy nueva
        let five_years_ago = now.saturating_sub(FIVE_YEARS_IN_SECONDS);

        // 2. Obtener la lista de préstamos
        let loan_list = self.user_loans.get(user_address);

        // 3. Iterar y filtrar
        for loan in loan_list.iter() {
            // ¡FILTRO DE 5 AÑOS!
            // Solo contamos préstamos OTORGADOS en los últimos 5 años
            if loan.timestamp_issued >= five_years_ago {
                total_loans_in_period += U256::from(1);

                if loan.is_paid {
                    paid_loans_in_period += U256::from(1);
                }
            }
        }

        // 4. Calcular porcentaje
        if total_loans_in_period == U256::ZERO {
            // Si no hay préstamos en los últimos 5 años, tiene 100% de cumplimiento
            // (no ha fallado en ningún pago). Esto es debatible, pero es un default seguro.
            return Ok(U256::from(100));
        }

        // Usamos multiplicación primero para preservar la precisión con enteros
        let percentage = (paid_loans_in_period * U256::from(100)) / total_loans_in_period;

        Ok(percentage)
    }
}