#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use alloc::vec::Vec;
use stylus_sdk::{
    alloy_primitives::{Address, Bytes, U256},
    prelude::*,
    storage::{StorageAddress, StorageMap},
    block, // Para obtener el timestamp
};

// --- Definición de la Interfaz para BankRegistry ---
// Esto le dice a Stylus cómo llamar al otro contrato.
sol_interface! {
    interface IBankRegistry {
        function is_trusted_bank(address bank_address) external view returns (bool);
    }
}

// --- Definición del Struct ---
// Usamos macros de Alloy para que sea serializable y almacenable
#[derive(Default, Debug, EthAbiType, EthAbiCodec, Clone)]
pub struct Attestation {
    bank_address: Address, // Quién lo reportó
    timestamp: U256,       // Cuándo se reportó
    data_type: Bytes,      // Tipo de dato (ej. "INGRESO_MENSUAL")
    value: U256,           // Un valor estandarizado
}

// --- Almacenamiento del Contrato ---
#[sol_storage]
#[entrypoint]
pub struct UserAttestations {
    /// Dirección del contrato BankRegistry
    bank_registry: StorageAddress,

    /// Mapping de dirección de usuario a un vector de sus atestados
    user_attestations: StorageMap<Address, Vec<Attestation>>,
}

// --- Lógica del Contrato ---
#[external]
impl UserAttestations {
    /// Constructor: se despliega con la dirección del BankRegistry
    pub fn new(registry_address: Address) -> Result<Self, Vec<u8>> {
        let mut contract = Self::default();
        contract.bank_registry.set(registry_address);
        Ok(contract)
    }

    /// Función que los bancos llaman para enviar un atestado
    pub fn submit_attestation(
        &mut self,
        user_address: Address,
        data_type: Bytes,
        value: U256,
    ) -> Result<(), Vec<u8>> {
        
        // --- Paso 1 y 2 (La Guardia) ---
        // Obtiene la dirección del banco que llama (msg.sender)
        let bank_address = msg::sender();
        
        // Carga la interfaz del registry
        let registry = IBankRegistry::new(self.bank_registry.get());

        // Llama al registry para verificar si el banco es confiable
        let is_trusted = registry.is_trusted_bank(self, bank_address)?;

        // Si no es confiable, revierte la transacción
        if !is_trusted {
            return Err(b"SENDER_NOT_TRUSTED_BANK".to_vec());
        }

        // --- Paso 3 (El Registro) ---
        let new_attestation = Attestation {
            bank_address,
            timestamp: block::timestamp(), // Timestamp actual
            data_type,
            value,
        };

        // Obtenemos la lista actual de atestados del usuario
        let mut attestations = self.user_attestations.get(user_address);
        
        // Agregamos el nuevo
        attestations.push(new_attestation);
        
        // Guardamos la lista actualizada
        self.user_attestations.insert(user_address, attestations);

        Ok(())
    }

    /// Función de solo-lectura para obtener todos los atestados de un usuario
    #[view]
    pub fn get_attestations(&self, user_address: Address) -> Result<Vec<Attestation>, Vec<u8>> {
        Ok(self.user_attestations.get(user_address))
    }

    /// Permite al dueño actualizar la dirección del registry
    pub fn set_registry_address(&mut self, new_address: Address) -> Result<(), Vec<u8>> {
        // Añadiríamos un chequeo de 'onlyOwner' aquí si fuera necesario
        // Por simplicidad, lo dejamos abierto, pero en producción deberías protegerlo.
        self.bank_registry.set(new_address);
        Ok(())
    }
}