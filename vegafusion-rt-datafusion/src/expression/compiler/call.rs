use vegafusion_core::error::{Result, VegaFusionError};
use crate::expression::compiler::builtin_functions::array::length::make_length_udf;
use crate::expression::compiler::builtin_functions::control_flow::if_fn::if_fn;
use crate::expression::compiler::builtin_functions::datetime::date_parts::{
    DATE_UDF, HOURS_UDF, MILLISECONDS_UDF, MINUTES_UDF, MONTH_UDF, SECONDS_UDF, UTCDATE_UDF,
    UTCHOURS_UDF, UTCMILLISECONDS_UDF, UTCMINUTES_UDF, UTCMONTH_UDF, UTCSECONDS_UDF, UTCYEAR_UDF,
    YEAR_UDF,
};
use crate::expression::compiler::builtin_functions::datetime::datetime::{
    datetime_transform, UTC_COMPONENTS,
};
use crate::expression::compiler::builtin_functions::math::isfinite::make_is_finite_udf;
use crate::expression::compiler::builtin_functions::math::isnan::make_is_nan_udf;
use crate::expression::compiler::builtin_functions::math::pow::make_pow_udf;
use crate::expression::compiler::builtin_functions::type_checking::isvalid::make_is_valid_udf;
use crate::expression::compiler::compile;
use crate::expression::compiler::config::CompilationConfig;
use crate::expression::compiler::utils::cast_to;
use datafusion::arrow::datatypes::DataType;
use datafusion::logical_plan::{DFSchema, Expr};
use datafusion::physical_plan::functions::BuiltinScalarFunction;
use datafusion::physical_plan::udf::ScalarUDF;
use std::collections::HashMap;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;
use vegafusion_core::proto_gen::expression::{Expression, CallExpression};

#[derive(Clone)]
pub enum VegaFusionCallable {
    /// A function that operates on the ESTree expression tree before compilation
    Macro(Arc<dyn Fn(&[Expression]) -> Result<Expression>>),

    /// A function that operates on the compiled arguments and produces a new expression.
    Transform(Arc<dyn Fn(&[Expr], &DFSchema) -> Result<Expr>>),

    /// Runtime function that is build in to DataFusion
    BuiltinScalarFunction {
        function: BuiltinScalarFunction,
        /// If Some, all arguments should be cast to provided type
        cast: Option<DataType>,
    },

    /// A custom runtime function that's not built into DataFusion
    ScalarUDF {
        udf: ScalarUDF,
        /// If Some, all arguments should be cast to provided type
        cast: Option<DataType>,
    },

    /// A custom macro that inputs a dataset, and uses that to generate the Expr tree
    ///
    /// e.g. `data('brush')` or  `vlSelectionTest('brush', datum, true)`
    // Data(Arc<dyn Fn(&DioriteTable, &[Expression], &DFSchema) -> Result<Expr>>),
    Data,

    /// A custom runtime function that operates on a scale dataset
    ///
    /// Placeholder for now
    Scale,
}

pub fn compile_scalar_arguments(
    node: &CallExpression,
    config: &CompilationConfig,
    schema: &DFSchema,
    cast: &Option<DataType>,
) -> Result<Vec<Expr>> {
    let mut args: Vec<Expr> = Vec::new();
    for arg in &node.arguments {
        let compiled_arg = compile(arg, config, Some(schema))?;
        let arg_expr = match cast {
            None => compiled_arg,
            Some(dtype) => cast_to(compiled_arg, dtype, schema)?,
        };
        args.push(arg_expr);
    }
    Ok(args)
}

pub fn compile_call(
    node: &CallExpression,
    config: &CompilationConfig,
    schema: &DFSchema,
) -> Result<Expr> {
    let callable = config.callable_scope.get(&node.callee).ok_or_else(|| {
        VegaFusionError::compilation(&format!("No global function named {}", &node.callee))
    })?;

    match callable {
        VegaFusionCallable::Macro(callable) => {
            // Apply macro then recursively compile
            let new_expr = callable(&node.arguments)?;
            compile(&new_expr, config, Some(schema))
        }
        VegaFusionCallable::ScalarUDF { udf, cast } => {
            let args = compile_scalar_arguments(node, config, schema, cast)?;
            Ok(Expr::ScalarUDF {
                fun: Arc::new(udf.clone()),
                args,
            })
        }
        VegaFusionCallable::BuiltinScalarFunction { function, cast } => {
            let args = compile_scalar_arguments(node, config, schema, cast)?;
            Ok(Expr::ScalarFunction {
                fun: function.clone(),
                args,
            })
        }
        // VeagFusionCallable::Data(callable) => {
        //     todo!()
        //     // First argument must be a string with name of the dataset
        //     if let Some(Expression::Literal(
        //                     Literal {
        //                         value: LiteralValue::String(arg), ..
        //                     })) = node.arguments.get(0)
        //     {
        //         if let Some(dataset) = config.data_scope.get(arg) {
        //             callable(dataset, &node.arguments[1..], schema)
        //         } else {
        //             diorite_bail!("No dataset named {}. Available: {:?}", arg, config.data_scope.keys())
        //         }
        //     } else {
        //         diorite_bail!(
        //             "The first argument to the {} function must be a literal \
        //             string with the name of a dataset",
        //             callee.name
        //         )
        //     }
        // }
        VegaFusionCallable::Transform(callable) => {
            let args = compile_scalar_arguments(node, config, schema, &None)?;
            callable(&args, schema)
        }
        _ => {
            todo!()
        }
    }
}

pub fn default_callables() -> HashMap<String, VegaFusionCallable> {
    let mut callables: HashMap<String, VegaFusionCallable> = HashMap::new();
    callables.insert("if".to_string(), VegaFusionCallable::Macro(Arc::new(if_fn)));

    // Numeric functions built into DataFusion with names that match Vega.
    // Cast arguments to Float64
    for fun_name in &[
        "abs", "acos", "asin", "atan", "ceil", "cos", "exp", "floor", "round", "sqrt", "tan",
    ] {
        let function = BuiltinScalarFunction::from_str(fun_name).unwrap();
        callables.insert(
            fun_name.to_string(),
            VegaFusionCallable::BuiltinScalarFunction {
                function,
                cast: Some(DataType::Float64),
            },
        );
    }

    // DataFusion ln is Vega log
    callables.insert(
        "log".to_string(),
        VegaFusionCallable::BuiltinScalarFunction {
            function: BuiltinScalarFunction::Ln,
            cast: Some(DataType::Float64),
        },
    );

    // Custom udfs
    callables.insert(
        "pow".to_string(),
        VegaFusionCallable::ScalarUDF {
            udf: make_pow_udf(),
            cast: Some(DataType::Float64),
        },
    );

    callables.insert(
        "isNaN".to_string(),
        VegaFusionCallable::ScalarUDF {
            udf: make_is_nan_udf(),
            cast: None,
        },
    );

    callables.insert(
        "isFinite".to_string(),
        VegaFusionCallable::ScalarUDF {
            udf: make_is_finite_udf(),
            cast: None,
        },
    );

    callables.insert(
        "isValid".to_string(),
        VegaFusionCallable::ScalarUDF {
            udf: make_is_valid_udf(),
            cast: None,
        },
    );

    callables.insert(
        "length".to_string(),
        VegaFusionCallable::ScalarUDF {
            udf: make_length_udf(),
            cast: None,
        },
    );

    // Date parts
    callables.insert(
        "year".to_string(),
        VegaFusionCallable::ScalarUDF {
            udf: YEAR_UDF.deref().clone(),
            cast: None,
        },
    );
    callables.insert(
        "month".to_string(),
        VegaFusionCallable::ScalarUDF {
            udf: MONTH_UDF.deref().clone(),
            cast: None,
        },
    );
    callables.insert(
        "date".to_string(),
        VegaFusionCallable::ScalarUDF {
            udf: DATE_UDF.deref().clone(),
            cast: None,
        },
    );
    callables.insert(
        "hours".to_string(),
        VegaFusionCallable::ScalarUDF {
            udf: HOURS_UDF.deref().clone(),
            cast: None,
        },
    );
    callables.insert(
        "minutes".to_string(),
        VegaFusionCallable::ScalarUDF {
            udf: MINUTES_UDF.deref().clone(),
            cast: None,
        },
    );
    callables.insert(
        "seconds".to_string(),
        VegaFusionCallable::ScalarUDF {
            udf: SECONDS_UDF.deref().clone(),
            cast: None,
        },
    );
    callables.insert(
        "milliseconds".to_string(),
        VegaFusionCallable::ScalarUDF {
            udf: MILLISECONDS_UDF.deref().clone(),
            cast: None,
        },
    );

    callables.insert(
        "utcyear".to_string(),
        VegaFusionCallable::ScalarUDF {
            udf: UTCYEAR_UDF.deref().clone(),
            cast: None,
        },
    );
    callables.insert(
        "utcmonth".to_string(),
        VegaFusionCallable::ScalarUDF {
            udf: UTCMONTH_UDF.deref().clone(),
            cast: None,
        },
    );
    callables.insert(
        "utcdate".to_string(),
        VegaFusionCallable::ScalarUDF {
            udf: UTCDATE_UDF.deref().clone(),
            cast: None,
        },
    );
    callables.insert(
        "utchours".to_string(),
        VegaFusionCallable::ScalarUDF {
            udf: UTCHOURS_UDF.deref().clone(),
            cast: None,
        },
    );
    callables.insert(
        "utcminutes".to_string(),
        VegaFusionCallable::ScalarUDF {
            udf: UTCMINUTES_UDF.deref().clone(),
            cast: None,
        },
    );
    callables.insert(
        "utcseconds".to_string(),
        VegaFusionCallable::ScalarUDF {
            udf: UTCSECONDS_UDF.deref().clone(),
            cast: None,
        },
    );
    callables.insert(
        "utcmilliseconds".to_string(),
        VegaFusionCallable::ScalarUDF {
            udf: UTCMILLISECONDS_UDF.deref().clone(),
            cast: None,
        },
    );

    // date time
    callables.insert(
        "datetime".to_string(),
        VegaFusionCallable::Transform(Arc::new(datetime_transform)),
    );
    callables.insert(
        "utc".to_string(),
        VegaFusionCallable::ScalarUDF {
            udf: UTC_COMPONENTS.deref().clone(),
            cast: Some(DataType::Int64),
        },
    );

    callables
}
