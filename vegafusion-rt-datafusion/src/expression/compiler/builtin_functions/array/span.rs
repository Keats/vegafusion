use datafusion::arrow::array::{new_null_array, Array, Int32Array, ListArray};
use datafusion::arrow::compute::kernels;
use datafusion::arrow::datatypes::{DataType, Field};
use datafusion::physical_plan::functions::{
    ReturnTypeFunction, ScalarFunctionImplementation, Signature, Volatility,
};
use datafusion::physical_plan::udf::ScalarUDF;
use datafusion::physical_plan::ColumnarValue;
use datafusion::scalar::ScalarValue;
use std::convert::TryFrom;
use std::sync::Arc;
use vegafusion_core::data::scalar::ScalarValueHelpers;

/// `span(array)`
///
/// Returns the span of array: the difference between the last and first elements,
/// or array[array.length-1] - array[0].
///
/// See https://vega.github.io/vega/docs/expressions/#span
pub fn make_span_udf() -> ScalarUDF {
    let span_fn: ScalarFunctionImplementation = Arc::new(|args: &[ColumnarValue]| {
        // Signature ensures there is a single argument
        let arg = &args[0];
        Ok(match arg {
            ColumnarValue::Scalar(value) => {
                println!("Span: {:?}", value);
                match value {
                    ScalarValue::Float64(_) => {
                        ColumnarValue::Scalar(ScalarValue::try_from(&DataType::Float64).unwrap())
                    }
                    ScalarValue::List(Some(arr), element_type) => {
                        match element_type.as_ref() {
                            DataType::Float64 => {
                                if arr.is_empty() {
                                    // Span of empty array is null
                                    ColumnarValue::Scalar(ScalarValue::try_from(&DataType::Float64).unwrap())
                                } else {
                                    let first = arr.first().unwrap().to_f64().unwrap();
                                    let last = arr.last().unwrap().to_f64().unwrap();
                                    ColumnarValue::Scalar(ScalarValue::from(last - first))
                                }
                            }
                            _ => {
                                panic!("Unexpected element type for span function: {}", element_type)
                            }
                        }
                    }
                    _ => {
                        panic!("Unexpected type passed to span: {}", value)
                    }
                }
            }
            ColumnarValue::Array(array) => {
                println!("Span: {:?}", array);
                todo!("Span on column not yet implemented")
                // match array.data_type() {
                //     DataType::Utf8 | DataType::LargeUtf8 => {
                //         // String length
                //         ColumnarValue::Array(kernels::length::length(array.as_ref()).unwrap())
                //     }
                //     DataType::FixedSizeList(_, n) => {
                //         // Use scalar length
                //         ColumnarValue::Scalar(ScalarValue::from(*n))
                //     }
                //     DataType::List(_) => {
                //         let array = array.as_any().downcast_ref::<ListArray>().unwrap();
                //         let offsets = array.value_offsets();
                //         let mut length_builder = Int32Array::builder(array.len());
                //
                //         for i in 0..array.len() {
                //             length_builder
                //                 .append_value((offsets[i + 1] - offsets[i]) as i32)
                //                 .unwrap();
                //         }
                //
                //         ColumnarValue::Array(Arc::new(length_builder.finish()))
                //     }
                //     _ => {
                //         // Array of i32 nulls
                //         ColumnarValue::Array(new_null_array(&DataType::Int32, array.len()))
                //     }
                // }
            }
        })
    });

    let return_type: ReturnTypeFunction = Arc::new(move |_| Ok(Arc::new(DataType::Float64)));
    ScalarUDF::new(
        "span",
        &Signature::uniform(
            1,
            vec![
                DataType::Float64,  // For null
                DataType::List(Box::new(Field::new("item", DataType::Float64, true)))
            ],
            Volatility::Immutable
        ),
        &return_type,
        &span_fn,
    )
}