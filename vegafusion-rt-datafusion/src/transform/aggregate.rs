/*
 * VegaFusion
 * Copyright (C) 2022 Jon Mease
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 * 
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU Affero General Public
 * License along with this program.
 * If not, see http://www.gnu.org/licenses/.
 */
use crate::expression::compiler::config::CompilationConfig;
use crate::transform::TransformTrait;
use datafusion::dataframe::DataFrame;
use datafusion::logical_plan::{avg, col, count, count_distinct, lit, max, min, sum, Expr};

use crate::expression::compiler::utils::to_numeric;
use async_trait::async_trait;
use std::sync::Arc;
use vegafusion_core::arrow::datatypes::DataType;
use vegafusion_core::error::{Result, ResultWithContext, VegaFusionError};
use vegafusion_core::proto::gen::transforms::{Aggregate, AggregateOp};
use vegafusion_core::task_graph::task_value::TaskValue;
use vegafusion_core::transform::aggregate::op_name;

#[async_trait]
impl TransformTrait for Aggregate {
    async fn eval(
        &self,
        dataframe: Arc<dyn DataFrame>,
        _config: &CompilationConfig,
    ) -> Result<(Arc<dyn DataFrame>, Vec<TaskValue>)> {
        let mut agg_exprs = Vec::new();
        for (i, (field, op)) in self.fields.iter().zip(self.ops.iter()).enumerate() {
            let column = if *op == AggregateOp::Count as i32 {
                // In Vega, the provided column is always ignored if op is 'count'.
                lit(0)
            } else {
                match field.as_str() {
                    "" => {
                        return Err(VegaFusionError::specification(&format!(
                            "Null field is not allowed for {:?} op",
                            op
                        )))
                    }
                    column => col(column),
                }
            };
            let numeric_column = || {
                to_numeric(column.clone(), dataframe.schema()).unwrap_or_else(|_| {
                    panic!("Failed to convert column {:?} to numeric data type", column)
                })
            };
            let op = AggregateOp::from_i32(*op).unwrap();

            let expr = match op {
                AggregateOp::Count => count(column),
                AggregateOp::Mean | AggregateOp::Average => avg(numeric_column()),
                AggregateOp::Min => min(numeric_column()),
                AggregateOp::Max => max(numeric_column()),
                AggregateOp::Sum => sum(numeric_column()),
                AggregateOp::Valid => {
                    let valid = Expr::Cast {
                        expr: Box::new(Expr::IsNotNull(Box::new(column))),
                        data_type: DataType::UInt64,
                    };
                    sum(valid)
                }
                AggregateOp::Missing => {
                    let missing = Expr::Cast {
                        expr: Box::new(Expr::IsNull(Box::new(column))),
                        data_type: DataType::UInt64,
                    };
                    sum(missing)
                }
                AggregateOp::Distinct => count_distinct(column),
                _ => {
                    return Err(VegaFusionError::specification(&format!(
                        "Unsupported aggregation op: {:?}",
                        op
                    )))
                }
            };

            // Apply alias
            let expr = if let Some(alias) = self.aliases.get(i).filter(|a| !a.is_empty()) {
                // Alias is a non-empty string
                expr.alias(alias)
            } else {
                expr.alias(&format!(
                    "{}_{}",
                    op_name(op),
                    (if field.is_empty() { "null" } else { field }).to_string(),
                ))
            };
            agg_exprs.push(expr)
        }

        let group_exprs: Vec<_> = self.groupby.iter().map(|c| col(c)).collect();

        let mut grouped_dataframe = dataframe
            .aggregate(group_exprs, agg_exprs)
            .with_context(|| "Failed to perform aggregate transform".to_string())?;

        // For determinism, sort result by grouping keys
        let sort_exprs: Vec<_> = self
            .groupby
            .iter()
            .map(|c| Expr::Sort {
                expr: Box::new(col(c)),
                asc: true,
                nulls_first: false,
            })
            .collect();
        if !sort_exprs.is_empty() {
            grouped_dataframe = grouped_dataframe.sort(sort_exprs)?;
        }

        Ok((grouped_dataframe, Vec::new()))
    }
}
