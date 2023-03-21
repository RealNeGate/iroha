//! Structures, traits and impls related to *runtime* `Validator`s.
//!
//! # Note
//!
//! Currently Iroha 2 has only builtin validators (see `core/src/smartcontracts/permissions`).
//! They are partly using API from this module.
//! In the future they will be replaced with *runtime validators* that use WASM.
//! The architecture of the new validators is quite different from the old ones.
//! That's why some parts of this module may not be used anywhere yet.
use iroha_data_model_derive::IdEqOrdHash;
use iroha_macro::FromVariant;

use super::*;
use crate::{
    account::Account,
    expression::Expression,
    isi::InstructionBox,
    model,
    query::QueryBox,
    transaction::{SignedTransaction, WasmSmartContract},
    ParseError,
};

model! {
    /// Identification of a [`Validator`].
    ///
    /// Consists of Validator's name and account (authority) id
    #[derive(Debug, Display, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Constructor, Getters, Decode, Encode, DeserializeFromStr, SerializeDisplay, IntoSchema)]
    #[display(fmt = "{name}%{owned_by}")]
    #[getset(get = "pub")]
    #[ffi_type]
    pub struct ValidatorId {
        /// Name given to validator by its creator.
        pub name: Name,
        /// Account that owns the validator.
        pub owned_by: <Account as Identifiable>::Id,
    }

    /// Permission validator that checks if an operation satisfies some conditions.
    ///
    /// Can be used with things like [`Transaction`]s,
    /// [`InstructionBox`]s, etc.
    #[derive(Debug, Display, Clone, IdEqOrdHash, Constructor, Getters, Decode, Encode, Deserialize, Serialize, IntoSchema)]
    #[allow(clippy::multiple_inherent_impl)]
    #[display(fmt = "{id}")]
    #[ffi_type]
    pub struct Validator {
        /// Identification of this [`Validator`].
        pub id: ValidatorId,
        /// Type of the validator
        #[getset(get = "pub")]
        pub validator_type: ValidatorType,
        /// WASM code of the validator
        // TODO: use another type like `WasmValidator`?
        #[getset(get = "pub")]
        pub wasm: WasmSmartContract,
    }
}

impl Registered for Validator {
    type With = Self;
}

impl core::str::FromStr for ValidatorId {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(ParseError {
                reason: "`ValidatorId` cannot be empty",
            });
        }

        let mut split = s.split('%');
        match (split.next(), split.next(), split.next()) {
            (Some(name), Some(account_id), None) => Ok(Self {
                name: name.parse()?,
                owned_by: account_id.parse()?,
            }),
            _ => Err(ParseError {
                reason: "Validator ID should have format `validator%account_id`",
            }),
        }
    }
}

model! {
    /// Type of validator
    #[derive(Debug, Display, Copy, Clone, PartialEq, Eq, Hash, Encode, Decode, Deserialize, Serialize, IntoSchema)]
    #[repr(u8)]
    #[ffi_type]
    pub enum ValidatorType {
        /// Validator checking [`SignedTransaction`]
        Transaction,
        /// Validator checking [`InstructionBox`]
        Instruction,
        /// Validator checking [`QueryBox`]
        Query,
        /// Validator checking [`Expression`]
        Expression,
    }
}

/// Operation for which the permission should be checked
pub trait NeedsPermission {
    /// Get the type of validator required to check the operation
    ///
    /// Accepts `self` because of the [`NeedsPermissionBox`]
    fn required_validator_type(&self) -> ValidatorType;
}

impl NeedsPermission for InstructionBox {
    fn required_validator_type(&self) -> ValidatorType {
        ValidatorType::Instruction
    }
}

impl NeedsPermission for QueryBox {
    fn required_validator_type(&self) -> ValidatorType {
        ValidatorType::Query
    }
}

// Expression might contain a query, therefore needs to be checked.
impl NeedsPermission for Expression {
    fn required_validator_type(&self) -> ValidatorType {
        ValidatorType::Expression
    }
}

model! {
    // TODO: Client doesn't need structures defined inside this macro. When dynamic linking is
    // implemented use: #[cfg(any(feature = "transparent_api", feature = "ffi_import"))]

    /// Boxed version of [`NeedsPermission`]
    #[derive(Debug, Display, Clone, PartialEq, Eq, FromVariant, Decode, Encode, Deserialize, Serialize)]
    #[ffi_type]
    pub enum NeedsPermissionBox {
        /// [`Transaction`] application operation
        Transaction(SignedTransaction),
        /// [`InstructionBox`] execution operation
        Instruction(InstructionBox),
        /// [`QueryBox`] execution operations
        Query(QueryBox),
        /// [`Expression`] evaluation operation
        Expression(Expression),
    }

    /// Validation verdict. All *runtime validators* should return this type.
    ///
    /// All operations are considered to be **valid** unless proven otherwise.
    /// Validators are allowed to either pass an operation to the next validator
    /// or to deny an operation.
    ///
    /// # Note
    ///
    /// There is no `Allow` variant (as well as it isn't a [`Result`] alias)
    /// because `Allow` and `Result` have a wrong connotation and suggest
    /// an incorrect interpretation of validators system.
    ///
    /// All operations are allowed by default.
    /// Validators are checking for operation **incorrectness**, not for operation correctness.
    #[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Deserialize, Serialize, IntoSchema)]
    pub enum Verdict {
        /// Operation is approved to pass to the next validator
        /// or to be executed if there are no more validators
        Pass,
        /// Operation is denied
        Deny(DenialReason),
    }
}

impl NeedsPermission for NeedsPermissionBox {
    fn required_validator_type(&self) -> ValidatorType {
        match self {
            NeedsPermissionBox::Transaction(_) => ValidatorType::Transaction,
            NeedsPermissionBox::Instruction(_) => ValidatorType::Instruction,
            NeedsPermissionBox::Query(_) => ValidatorType::Query,
            NeedsPermissionBox::Expression(_) => ValidatorType::Expression,
        }
    }
}

impl Verdict {
    /// Returns [`Deny`] if the verdict is [`Deny`], otherwise returns `other`.
    ///
    /// Arguments passed to and are eagerly evaluated;
    /// if you are passing the result of a function call,
    /// it is recommended to use [`and_then`](Verdict::and_then()), which is lazily evaluated.
    ///
    /// [`Deny`]: Verdict::Deny
    #[must_use]
    pub fn and(self, other: Verdict) -> Verdict {
        match self {
            Verdict::Pass => other,
            Verdict::Deny(_) => self,
        }
    }

    /// Returns [`Deny`] if the verdict is [`Deny`], otherwise calls `f` and returns the result.
    ///
    /// [`Deny`]: Verdict::Deny
    #[must_use]
    pub fn and_then<F>(self, f: F) -> Verdict
    where
        F: FnOnce() -> Verdict,
    {
        match self {
            Verdict::Pass => f(),
            Verdict::Deny(_) => self,
        }
    }
}

impl From<Verdict> for Result<(), DenialReason> {
    fn from(verdict: Verdict) -> Self {
        match verdict {
            Verdict::Pass => Ok(()),
            Verdict::Deny(reason) => Err(reason),
        }
    }
}

/// Reason for denying the execution of a particular instruction.
pub type DenialReason = String;