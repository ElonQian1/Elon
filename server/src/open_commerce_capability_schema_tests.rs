use serde_json::json;

use crate::open_commerce_capability_schema::{
    validate_input, validate_input_schema, validate_output, validate_output_schema,
};

fn order_quote_schema() -> serde_json::Value {
    json!({
        "type":"object",
        "required":["items"],
        "properties":{
            "items":{
                "type":"array",
                "minItems":1,
                "maxItems":50,
                "items":{
                    "type":"object",
                    "required":["product_id","quantity"],
                    "properties":{
                        "product_id":{"type":"string","format":"uuid"},
                        "quantity":{"type":"integer","minimum":1,"maximum":100}
                    },
                    "additionalProperties":false
                }
            },
            "note":{"type":"string","maxLength":500}
        },
        "additionalProperties":false
    })
}

#[test]
fn accepted_profile_validates_runtime_order_input() {
    let schema = order_quote_schema();
    validate_input_schema(&schema).unwrap();
    validate_input(
        &schema,
        &json!({
            "items":[{
                "product_id":"24bdeda3-d303-4d65-bfed-38d50b8a10aa",
                "quantity":2
            }],
            "note":"少糖"
        }),
    )
    .unwrap();
}

#[test]
fn violations_report_only_path_and_keyword() {
    let schema = order_quote_schema();
    let error = validate_input(
        &schema,
        &json!({
            "items":[{
                "product_id":"sensitive-product-value",
                "quantity":0
            }]
        }),
    )
    .unwrap_err();
    assert_eq!(error.path, "$.items[0].product_id");
    assert_eq!(error.code, "format");
    assert!(!error.to_string().contains("sensitive-product-value"));

    let error = validate_input(
        &schema,
        &json!({
            "items":[{
                "product_id":"24bdeda3-d303-4d65-bfed-38d50b8a10aa",
                "quantity":0
            }]
        }),
    )
    .unwrap_err();
    assert_eq!(error.path, "$.items[0].quantity");
    assert_eq!(error.code, "minimum");
}

#[test]
fn required_and_additional_properties_fail_closed() {
    let schema = order_quote_schema();
    let missing = validate_input(&schema, &json!({})).unwrap_err();
    assert_eq!(missing.path, "$.items");
    assert_eq!(missing.code, "required");

    let extra = validate_input(
        &schema,
        &json!({
            "items":[{
                "product_id":"24bdeda3-d303-4d65-bfed-38d50b8a10aa",
                "quantity":1,
                "admin":true
            }]
        }),
    )
    .unwrap_err();
    assert_eq!(extra.path, "$.items[0].admin");
    assert_eq!(extra.code, "additionalProperties");
}

#[test]
fn unsupported_schema_keywords_are_rejected_before_publication() {
    let error = validate_input_schema(&json!({
        "type":"object",
        "$ref":"https://example.com/remote-schema.json"
    }))
    .unwrap_err();
    assert!(error.to_string().contains("不支持的关键字 $ref"));

    let error = validate_input_schema(&json!({"type":"array"})).unwrap_err();
    assert!(error.to_string().contains("根节点 type 必须是 object"));
}

#[test]
fn output_contract_supports_const_enum_and_scalar_values() {
    let schema = json!({
        "type":"object",
        "required":["status","currency","total_minor"],
        "properties":{
            "status":{"const":"quoted"},
            "currency":{"type":"string","enum":["CNY"]},
            "total_minor":{"type":"integer","minimum":0}
        },
        "additionalProperties":false
    });
    validate_output_schema(&schema).unwrap();
    validate_output(
        &schema,
        &json!({"status":"quoted","currency":"CNY","total_minor":2600}),
    )
    .unwrap();
    let error = validate_output(
        &schema,
        &json!({"status":"paid","currency":"CNY","total_minor":2600}),
    )
    .unwrap_err();
    assert_eq!(error.path, "$.status");
    assert_eq!(error.code, "const");
}

#[test]
fn empty_schema_remains_compatible_with_existing_capabilities() {
    validate_input_schema(&json!({})).unwrap();
    validate_output_schema(&json!({})).unwrap();
    validate_input(&json!({}), &json!({"legacy":true})).unwrap();
    validate_output(&json!({}), &json!(["legacy", "result"])).unwrap();
}
