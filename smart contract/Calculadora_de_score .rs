#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use alloc::vec::Vec;
use stylus_sdk::{
    alloy_primitives::{Address, Bytes, U256},
    prelude::*,
    storage::StorageAddress,
};

// --- Definición del Struct (debe coincidir con el de UserAttestations) ---
#[derive(Default, Debug, EthAbiType, EthAbiCodec, Clone)]
pub struct Attestation {
    bank_address: Address,
    timestamp: U256,
    data_type: Bytes,
    value: U256,
}

// --- Definición de la Interfaz para UserAttestations ---
sol_interface! {
    interface IAttestations {
        function get_attestations(address user_address) external view returns (Vec<Attestation>);
    }
}

// --- Almacenamiento del Contrato ---
#[sol_storage]
#[entrypoint]
pub struct ScoreCalculator {
    /// Dirección del contrato UserAttestations
    attestations_contract: StorageAddress,
}

// --- Lógica del Contrato ---
#[external]
impl ScoreCalculator {
    /// Constructor: se despliega con la dirección de UserAttestations
    pub fn new(attestations_address: Address) -> Result<Self, Vec<u8>> {
        let mut contract = Self::default();
        contract.attestations_contract.set(attestations_address);
        Ok(contract)
    }

    /// La función principal que calcula el score
    #[view]
    pub fn calculate_score(
        &self,
        user_address: Address,
        ppa_factor: U256, // El frontend pasa este valor
    ) -> Result<U256, Vec<u8>> {
        
        // --- Paso 1: Obtener Atestados ---
        let attestations_loader = IAttestations::new(self.attestations_contract.get());
        let attestations = attestations_loader.get_attestations(self, user_address)?;

        // --- Paso 2: Iterar y Calcular Score Bruto ---
        // ¡Aquí es donde pones tu fórmula de "weighted average"!
        // Esto es solo un ejemplo.
        let mut score_bruto = U256::from(0);

        for att in attestations.iter() {
            // Ejemplo de lógica simple basada en el data_type
            if att.data_type == "INGRESO_ALTO".as_bytes() {
                // Suma el valor (ej. 100 puntos)
                score_bruto += att.value; 
            } else if att.data_type == "NIVEL_DEUDA".as_bytes() {
                // Resta el valor (ej. 50 puntos)
                if score_bruto >= att.value {
                    score_bruto -= att.value;
                } else {
                    score_bruto = U256::ZERO; // Evitar underflow
                }
            } else if att.data_type == "TASA_AHORRO_ALTA".as_bytes() {
                score_bruto += att.value;
            }
            // ... agrega más lógica de negocio aquí
        }

        // --- Paso 3: Aplicar PPA ---
        // Evitar división por cero
        if ppa_factor == U256::ZERO {
            // Decide qué retornar en este caso, 0 tiene sentido.
            return Ok(U256::ZERO); 
        }

        // NOTA: U256 hace división de enteros. 
        // Para manejar decimales (como 0.6), debes usar aritmética
        // de punto fijo.
        //
        // Ejemplo: Si PPA=0.6, el frontend debe enviar 6 y un 
        // factor de 10. (o 60 y factor 100).
        //
        // Asumamos que el frontend envía el PPA multiplicado por 100.
        // Ej: Para 0.6, envía `ppa_factor = U256::from(60)`
        // Ej: Para 1.2, envía `ppa_factor = U256::from(120)`
        //
        // final_score = (score_bruto * 100) / ppa_factor
        
        let precision_factor = U256::from(100);
        
        // Multiplicamos *primero* para preservar la precisión
        let final_score = (score_bruto * precision_factor) / ppa_factor;

        // --- Paso 4: Devolver Score Final ---
        Ok(final_score)
    }

    /// Permite al dueño actualizar la dirección del contrato de atestados
    pub fn set_attestations_address(&mut self, new_address: Address) -> Result<(), Vec<u8>> {
        // Proteger con 'onlyOwner' en producción
        self.attestations_contract.set(new_address);
        Ok(())
    }
}