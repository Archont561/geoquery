//! Attribute filters: the half of a query's predicate that CQL2 already solves.
//!
//! The tree here is CQL2-*inspired*, not a CQL2 clone. It borrows the concepts — property
//! references, typed literals, comparison, boolean composition — because OGC API Features
//! standardises CQL2 and the STAC Filter extension speaks CQL2 JSON, so every adapter has
//! somewhere to compile to. It does not borrow the encoding: a `GeoQuery` also carries
//! federation scope, ranking and execution hints that CQL2 has no way to say, which is the
//! whole reason this crate has an AST instead of passing a CQL2 document through.
//!
//! Spatial and temporal *predicates* are deliberately not here. They are [`crate::query`]'s
//! `spatial` and `temporal` members, because a geometry predicate is a different kind of
//! thing to push down — a source may support one and not the other, and the planner needs
//! to see them separately to decide. A compiler merges all three on the way out, since CQL2
//! treats every predicate uniformly. What *is* here is the temporal literal, so a `datetime`
//! queryable can be compared like any other property.
//!
//! Two ways to be wrong, and the split is load-bearing:
//!
//! - A node whose **shape** is wrong cannot be built. An operator nothing can compile, a
//!   list where a scalar belongs, a member this version does not model: all refused by
//!   [`Deserialize`], so the variants below are only ever inhabited by coherent nodes.
//! - A node that parsed but cannot **mean** anything — an empty branch, a membership test
//!   against nothing — is refused by [`FilterExpr::validate`], which reports every problem
//!   in the tree rather than the first.

use std::fmt;

use chrono::{DateTime, Utc};
use serde::de::{self, MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use thiserror::Error;
use ts_rs::TS;

/// The members a filter node may carry, in the one place every reader of them can see.
const AND: &str = "and";
const OR: &str = "or";
const NOT: &str = "not";
const FIELD: &str = "field";
const OP: &str = "op";
const VALUE: &str = "value";
const TIMESTAMP: &str = "timestamp";
const IN: &str = "in";
const LIKE: &str = "like";

const NODE_MEMBERS: &[&str] = &[AND, OR, NOT, FIELD, OP, VALUE];

/// How deep a filter may nest before it is refused.
///
/// Every compiler in `.knowledge/query/filters.md` walks this tree recursively, and so does
/// [`FilterExpr::validate`]. The cap is far past anything a person writes by hand and far
/// short of a stack, so a generated filter that has run away produces an error naming the
/// limit rather than aborting the process that was asked to compile it.
pub const MAX_FILTER_DEPTH: usize = 32;

/// A comparison between a property and a literal.
///
/// Closed, unlike the predicate enums in [`crate::query`]. Those are open because a
/// *service* may support an operation this version has not heard of, and refusing to carry
/// it would refuse queries a newer source would have answered. An operator is the other
/// way round: this engine is the one that has to emit it, so an operator it cannot name is
/// an operator it cannot compile, and accepting it would only move the failure to whichever
/// adapter trusted the tree it was handed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TS)]
// Written out rather than derived, because the codec is hand-written and ts-rs records no
// dependencies for a type override: a reference to `CompareOp` here would emit a name the
// generated TypeScript never imports. `every_comparison_operator_has_one_spelling_on_the_wire`
// in `tests/filter.rs` is what keeps this list and `as_str` below from drifting apart.
#[ts(type = r#""=" | "<>" | "<" | ">" | "<=" | ">=""#)]
pub enum CompareOp {
    /// Equal.
    Eq,
    /// Not equal.
    Ne,
    /// Less than.
    Lt,
    /// Greater than.
    Gt,
    /// Less than or equal.
    Le,
    /// Greater than or equal.
    Ge,
}

impl CompareOp {
    /// The one spelling this operator has on the wire.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Eq => "=",
            Self::Ne => "<>",
            Self::Lt => "<",
            Self::Gt => ">",
            Self::Le => "<=",
            Self::Ge => ">=",
        }
    }

    /// Read a wire spelling, or nothing if no operator has it.
    pub(crate) fn from_wire(text: &str) -> Option<Self> {
        match text {
            "=" => Some(Self::Eq),
            "<>" => Some(Self::Ne),
            "<" => Some(Self::Lt),
            ">" => Some(Self::Gt),
            "<=" => Some(Self::Le),
            ">=" => Some(Self::Ge),
            _ => None,
        }
    }

    /// Whether this operator ranks its operands rather than merely matching them.
    const fn orders_its_operands(self) -> bool {
        matches!(self, Self::Lt | Self::Gt | Self::Le | Self::Ge)
    }
}

impl fmt::Display for CompareOp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// The complaint for an operator this engine has no compiler for.
///
/// Shared by the two places that read one — a bare operator and the `op` member of a
/// comparison — because two copies of a message are two messages waiting to disagree.
fn unknown_operator(text: &str) -> String {
    format!(
        "`{text}` is not an operator this engine can compile; \
         use =, <>, <, >, <=, >=, {IN} or {LIKE}"
    )
}

impl Serialize for CompareOp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for CompareOp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;
        Self::from_wire(&text).ok_or_else(|| de::Error::custom(unknown_operator(&text)))
    }
}

/// A literal a property is compared against.
///
/// The four types are the ones `.knowledge/query/filters.md` gives queryables, minus
/// `geometry`, which belongs to the spatial predicate and not to an attribute test. Nothing
/// here can hold a value JSON cannot carry — there is no `From<f64>` because a `f64` may be
/// NaN and a JSON number may not, so build those through [`serde_json::Number::from_f64`]
/// and let it refuse the ones that have no wire form.
#[derive(Debug, Clone, PartialEq, Eq, TS)]
#[ts(type = "string | number | boolean | { timestamp: string }")]
pub enum FilterValue {
    /// Text.
    String(String),
    /// A JSON number, kept exactly as written so an integer does not come back a float.
    Number(serde_json::Number),
    /// A flag.
    Boolean(bool),
    /// An instant, written `{ "timestamp": "…" }` as CQL2 JSON writes it.
    Timestamp(DateTime<Utc>),
}

impl FilterValue {
    /// The queryable type this literal can be compared against.
    ///
    /// The vocabulary is the corpus's own — `string`, `number`, `boolean`, `datetime` — so
    /// a planner checking a filter against a service's declared queryables compares like
    /// with like instead of translating between two spellings of the same four types.
    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::String(_) => "string",
            Self::Number(_) => "number",
            Self::Boolean(_) => "boolean",
            Self::Timestamp(_) => "datetime",
        }
    }

    /// Read a literal out of parsed JSON, saying what was wrong if it is not one.
    fn from_json(value: Value) -> Result<Self, String> {
        match value {
            Value::String(text) => Ok(Self::String(text)),
            Value::Number(number) => Ok(Self::Number(number)),
            Value::Bool(flag) => Ok(Self::Boolean(flag)),
            Value::Object(members) => Self::timestamp_from_json(members),
            Value::Null | Value::Array(_) => Err(format!(
                "a filter literal is a string, a number, a boolean or \
                 {{ \"{TIMESTAMP}\": \"<instant>\" }}, and this is {}",
                describe(&value)
            )),
        }
    }

    /// Read the one object form a literal has.
    fn timestamp_from_json(members: serde_json::Map<String, Value>) -> Result<Self, String> {
        let mut entries = members.into_iter();
        match (entries.next(), entries.next()) {
            (Some((key, Value::String(text))), None) if key == TIMESTAMP => {
                crate::instant::read(&text)
                    .map(Self::Timestamp)
                    .ok_or_else(|| crate::instant::unreadable_message(&text))
            }
            _ => Err(format!(
                "the only object a filter literal may be is \
                 {{ \"{TIMESTAMP}\": \"<instant>\" }}"
            )),
        }
    }
}

/// Name a JSON value's kind for an error message.
fn describe(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "a list",
        Value::Object(_) => "an object",
    }
}

impl From<&str> for FilterValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<String> for FilterValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<bool> for FilterValue {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl From<i64> for FilterValue {
    fn from(value: i64) -> Self {
        Self::Number(value.into())
    }
}

impl From<serde_json::Number> for FilterValue {
    fn from(value: serde_json::Number) -> Self {
        Self::Number(value)
    }
}

impl From<DateTime<Utc>> for FilterValue {
    fn from(value: DateTime<Utc>) -> Self {
        Self::Timestamp(value)
    }
}

impl Serialize for FilterValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::String(text) => serializer.serialize_str(text),
            Self::Number(number) => number.serialize(serializer),
            Self::Boolean(flag) => serializer.serialize_bool(*flag),
            Self::Timestamp(instant) => {
                let mut wrapper = serializer.serialize_map(Some(1))?;
                wrapper.serialize_entry(TIMESTAMP, &crate::instant::write(*instant))?;
                wrapper.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for FilterValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Through `Value` rather than a visitor over the raw stream: the wire format is
        // JSON, the crate already carries `serde_json`, and the alternative needs a
        // `visit_f64` arm that has to refuse a NaN no JSON document can contain.
        let value = Value::deserialize(deserializer)?;
        Self::from_json(value).map_err(de::Error::custom)
    }
}

/// One node of an attribute filter.
///
/// `IN` and `LIKE` are their own variants rather than members of [`CompareOp`] because they
/// do not take the operand a comparison takes: one wants a list, the other a pattern. Making
/// them separate is what lets a compiler walk this tree without ever asking whether the
/// operand it was handed suits the operator it was given — the pairing was settled when the
/// node was built. `NOT IN` and `NOT LIKE` need no variants at all; they are [`Self::Not`]
/// around the positive test, which compiles to the same thing.
#[derive(Debug, Clone, PartialEq, Eq, TS)]
// Self-reference is legal in a TypeScript alias through an object property, so the tree
// types itself. The operator and literal unions are spelled out rather than referenced
// because ts-rs records no dependencies for a type override and the generated module would
// not import them. `tests/filter.rs` pins both lists.
#[ts(
    type = "{ and: FilterExpr[] } | { or: FilterExpr[] } | { not: FilterExpr } \
| { field: string, op: \"=\" | \"<>\" | \"<\" | \">\" | \"<=\" | \">=\", \
value: string | number | boolean | { timestamp: string } } \
| { field: string, op: \"in\", \
value: (string | number | boolean | { timestamp: string })[] } \
| { field: string, op: \"like\", value: string }"
)]
pub enum FilterExpr {
    /// Every argument must hold.
    And(Vec<FilterExpr>),
    /// At least one argument must hold.
    Or(Vec<FilterExpr>),
    /// The argument must not hold.
    Not(Box<FilterExpr>),
    /// A property compared against a literal.
    Compare {
        /// The property being tested.
        field: String,
        /// How it is compared.
        op: CompareOp,
        /// What it is compared against.
        value: FilterValue,
    },
    /// A property that must be one of a list of literals.
    In {
        /// The property being tested.
        field: String,
        /// The candidates it may equal.
        values: Vec<FilterValue>,
    },
    /// A property matched against a pattern, in the SQL `LIKE` dialect CQL2 inherits.
    Like {
        /// The property being tested.
        field: String,
        /// The pattern, where `%` stands for any run of characters and `_` for one.
        pattern: String,
    },
}

impl FilterExpr {
    /// Check every rule that can be checked without asking a source anything.
    ///
    /// Shape is already settled — a parsed node is a coherent one — so what is left is
    /// meaning: a branch with nothing in it, a membership test nothing can satisfy, a
    /// property with no name. Every problem is reported rather than the first, for the same
    /// reason [`crate::query::GeoQuery::validate`] does it: an agent repairing a generated
    /// filter wants the list, and one problem per attempt turns a fix into a conversation.
    ///
    /// # Errors
    ///
    /// Returns every way the filter cannot be compiled, in the order the tree is walked.
    pub fn validate(&self) -> Result<(), Vec<FilterValidationError>> {
        let mut problems = Vec::new();
        self.collect_problems(1, &mut problems);
        if problems.is_empty() {
            Ok(())
        } else {
            Err(problems)
        }
    }

    pub(crate) fn collect_problems(&self, depth: usize, problems: &mut Vec<FilterValidationError>) {
        if depth > MAX_FILTER_DEPTH {
            // Reported once and not descended into: the rest of the branch is more of the
            // same, and a caller does not need the limit restated for every level past it.
            problems.push(FilterValidationError::TooDeep {
                limit: MAX_FILTER_DEPTH,
            });
            return;
        }

        match self {
            Self::And(arguments) | Self::Or(arguments) => {
                if arguments.is_empty() {
                    problems.push(FilterValidationError::EmptyBranch {
                        op: self.branch_name().to_owned(),
                    });
                }
                for argument in arguments {
                    argument.collect_problems(depth + 1, problems);
                }
            }
            Self::Not(argument) => argument.collect_problems(depth + 1, problems),
            Self::Compare { field, op, value } => {
                collect_field_problems(field, problems);
                if op.orders_its_operands() && matches!(value, FilterValue::Boolean(_)) {
                    problems.push(FilterValidationError::BooleanIsNotOrdered {
                        field: field.clone(),
                        op: op.to_string(),
                    });
                }
            }
            Self::In { field, values } => {
                collect_field_problems(field, problems);
                collect_membership_problems(field, values, problems);
            }
            Self::Like { field, .. } => collect_field_problems(field, problems),
        }
    }

    /// The wire name of a boolean branch, for the error that names one.
    fn branch_name(&self) -> &'static str {
        match self {
            Self::Or(_) => OR,
            _ => AND,
        }
    }
}

/// A property reference has to name something.
fn collect_field_problems(field: &str, problems: &mut Vec<FilterValidationError>) {
    if field.trim().is_empty() {
        problems.push(FilterValidationError::FieldUnnamed);
    }
}

/// A membership test has to offer candidates, and they have to be one type.
fn collect_membership_problems(
    field: &str,
    values: &[FilterValue],
    problems: &mut Vec<FilterValidationError>,
) {
    let Some(first) = values.first() else {
        problems.push(FilterValidationError::EmptyMembership {
            field: field.to_owned(),
        });
        return;
    };

    // The first odd one out is enough: a list with three types in it has one mistake in
    // it, not two, and naming the type the list started as is what makes it findable.
    if let Some(odd) = values
        .iter()
        .find(|value| value.type_name() != first.type_name())
    {
        problems.push(FilterValidationError::MixedMembershipTypes {
            field: field.to_owned(),
            first: first.type_name().to_owned(),
            found: odd.type_name().to_owned(),
        });
    }
}

/// Everything a filter can get wrong once it has parsed.
///
/// One variant per rule rather than a message, for the same reason the rest of this crate
/// does it: the caller is usually a program deciding what to do, and the wording belongs to
/// whoever shows it to a person.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum FilterValidationError {
    /// A boolean branch has no arguments.
    #[error("boolean operator `{op}` needs at least one argument")]
    EmptyBranch {
        /// The branch that was empty.
        op: String,
    },
    /// A membership test offers no candidates.
    #[error("`{field}` is tested against an empty list, which no result can satisfy")]
    EmptyMembership {
        /// The property being tested.
        field: String,
    },
    /// A membership test spans more than one literal type.
    #[error("`{field}` is tested against a list that starts {first} and then holds {found}")]
    MixedMembershipTypes {
        /// The property being tested.
        field: String,
        /// The type the list started as.
        first: String,
        /// The type that did not match it.
        found: String,
    },
    /// An ordering comparison was asked for against a value that has no order.
    #[error("operator `{op}` ranks its operands and a boolean has no rank: `{field}`")]
    BooleanIsNotOrdered {
        /// The property being tested.
        field: String,
        /// The operator that was asked for.
        op: String,
    },
    /// A property reference names nothing.
    #[error("a filter names the property it tests, and this one is blank")]
    FieldUnnamed,
    /// The tree nests deeper than anything downstream will walk.
    #[error("the filter nests deeper than {limit} levels")]
    TooDeep {
        /// The limit that was passed.
        limit: usize,
    },
}

impl Serialize for FilterExpr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::And(arguments) => single_entry(serializer, AND, arguments),
            Self::Or(arguments) => single_entry(serializer, OR, arguments),
            Self::Not(argument) => single_entry(serializer, NOT, argument),
            Self::Compare { field, op, value } => comparison(serializer, field, op, value),
            Self::In { field, values } => comparison(serializer, field, IN, values),
            Self::Like { field, pattern } => comparison(serializer, field, LIKE, pattern),
        }
    }
}

/// Write `{ "<name>": <argument> }`, the shape every boolean branch has.
fn single_entry<S, T>(serializer: S, name: &str, argument: &T) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Serialize + ?Sized,
{
    let mut node = serializer.serialize_map(Some(1))?;
    node.serialize_entry(name, argument)?;
    node.end()
}

/// Write `{ "field": …, "op": …, "value": … }`, the shape every leaf has.
fn comparison<S, O, T>(serializer: S, field: &str, op: &O, value: &T) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    O: Serialize + ?Sized,
    T: Serialize + ?Sized,
{
    let mut node = serializer.serialize_map(Some(3))?;
    node.serialize_entry(FIELD, field)?;
    node.serialize_entry(OP, op)?;
    node.serialize_entry(VALUE, value)?;
    node.end()
}

impl<'de> Deserialize<'de> for FilterExpr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(FilterExprVisitor)
    }
}

struct FilterExprVisitor;

impl<'de> Visitor<'de> for FilterExprVisitor {
    type Value = FilterExpr;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .write_str("a filter node: `and`, `or`, `not`, or a `field`/`op`/`value` comparison")
    }

    fn visit_map<A>(self, mut members: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        // The branch is built as it is read and kept in one slot with the name it came
        // from. Three slots and a count afterwards would say the same thing and leave a
        // fourth case — none of them filled, inside a block that knows one of them is —
        // for the compiler to worry about and for nothing to ever reach.
        let mut branch: Option<(&'static str, FilterExpr)> = None;
        let mut field: Option<String> = None;
        let mut op: Option<String> = None;
        // Buffered rather than read straight into a `FilterValue`: what `value` is allowed
        // to be depends on `op`, and JSON does not promise to hand them over in that order.
        let mut value: Option<Value> = None;

        while let Some(key) = members.next_key::<String>()? {
            match key.as_str() {
                AND => branch_once(&mut branch, AND, FilterExpr::And(members.next_value()?))?,
                OR => branch_once(&mut branch, OR, FilterExpr::Or(members.next_value()?))?,
                NOT => branch_once(&mut branch, NOT, FilterExpr::Not(members.next_value()?))?,
                FIELD => once(&mut field, FIELD, members.next_value()?)?,
                OP => once(&mut op, OP, members.next_value()?)?,
                VALUE => once(&mut value, VALUE, members.next_value()?)?,
                unknown => return Err(de::Error::unknown_field(unknown, NODE_MEMBERS)),
            }
        }

        let leaf_member = [
            (FIELD, field.is_some()),
            (OP, op.is_some()),
            (VALUE, value.is_some()),
        ]
        .into_iter()
        .find_map(|(name, present)| present.then_some(name));

        if let Some((name, expression)) = branch {
            if let Some(member) = leaf_member {
                return Err(de::Error::custom(format!(
                    "a filter node is either `{name}` or a comparison, and this one has \
                     `{name}` alongside `{member}`"
                )));
            }
            return Ok(expression);
        }

        if leaf_member.is_none() {
            return Err(de::Error::custom(
                "a filter node is empty: it names neither `and`, `or`, `not` nor a \
                 `field`/`op`/`value` comparison",
            ));
        }

        let field = field.ok_or_else(|| de::Error::missing_field(FIELD))?;
        let op = op.ok_or_else(|| de::Error::missing_field(OP))?;
        let value = value.ok_or_else(|| de::Error::missing_field(VALUE))?;
        leaf_from_parts(field, &op, value)
    }
}

/// Record the one boolean branch a node may have.
///
/// Two copies of the same branch and two different branches are both refusals, and they
/// are different refusals: the first is a document that repeated itself, the second is one
/// that asked for two things at once.
fn branch_once<E>(
    slot: &mut Option<(&'static str, FilterExpr)>,
    name: &'static str,
    found: FilterExpr,
) -> Result<(), E>
where
    E: de::Error,
{
    match slot {
        Some((existing, _)) if *existing == name => Err(E::duplicate_field(name)),
        Some((existing, _)) => Err(E::custom(format!(
            "a filter node names one operator, and this one has both `{existing}` and `{name}`"
        ))),
        None => {
            *slot = Some((name, found));
            Ok(())
        }
    }
}

/// Record a member, refusing the second copy of one.
fn once<T, E>(slot: &mut Option<T>, name: &'static str, found: T) -> Result<(), E>
where
    E: de::Error,
{
    if slot.is_some() {
        return Err(E::duplicate_field(name));
    }
    *slot = Some(found);
    Ok(())
}

/// Pair an operator with the operand it takes, or say why the two do not go together.
fn leaf_from_parts<E>(field: String, op: &str, value: Value) -> Result<FilterExpr, E>
where
    E: de::Error,
{
    match op {
        IN => match value {
            Value::Array(items) => {
                let values = items
                    .into_iter()
                    .map(|item| FilterValue::from_json(item).map_err(E::custom))
                    .collect::<Result<Vec<_>, E>>()?;
                Ok(FilterExpr::In { field, values })
            }
            other => Err(E::custom(format!(
                "`{VALUE}` for operator `{IN}` is a list of literals, and this is {}",
                describe(&other)
            ))),
        },
        LIKE => match value {
            Value::String(pattern) => Ok(FilterExpr::Like { field, pattern }),
            other => Err(E::custom(format!(
                "`{VALUE}` for operator `{LIKE}` is a string pattern, and this is {}",
                describe(&other)
            ))),
        },
        _ => {
            let op = CompareOp::from_wire(op).ok_or_else(|| E::custom(unknown_operator(op)))?;
            if let Value::Array(_) = value {
                return Err(E::custom(format!(
                    "`{VALUE}` for operator `{op}` is a single literal, not a list; \
                     use `{IN}` to test against several"
                )));
            }
            Ok(FilterExpr::Compare {
                field,
                op,
                value: FilterValue::from_json(value).map_err(E::custom)?,
            })
        }
    }
}
