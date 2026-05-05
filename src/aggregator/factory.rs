use crate::aggregator::Aggregator;
use crate::aggregator::avg::AvgAgg;
use crate::aggregator::count::CountAgg;
use crate::aggregator::max::MaxAgg;
use crate::aggregator::min::MinAgg;
use crate::aggregator::sum::SumAgg;
use crate::parser::ast::AggFunc;
use crate::processors::processor::ExecutionError;
use crate::storage::schema::DataType;

/// Build a runtime aggregator from a parsed `AggFunc` and the input column's
/// `DataType`. Called once per aggregate expression while building the plan.
pub fn build(func: AggFunc, input: DataType) -> Result<Box<dyn Aggregator>, ExecutionError> {
    match func {
        AggFunc::Sum   => Ok(Box::new(SumAgg::new(input)?)),
        AggFunc::Max   => Ok(Box::new(MaxAgg::new(input)?)),
        AggFunc::Min   => Ok(Box::new(MinAgg::new(input)?)),
        AggFunc::Avg   => Ok(Box::new(AvgAgg::new(input)?)),
        AggFunc::Count => Ok(Box::new(CountAgg::new())),
    }
}
