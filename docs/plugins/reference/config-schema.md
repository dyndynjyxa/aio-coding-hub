# 插件配置 Schema

插件可以在 `plugin.json` 中声明 `configSchema`。它使用 JSON Schema 的一个受支持子集，由宿主负责渲染配置表单并在保存前后校验。

支持的标量类型：

- `string`
- `number`
- `integer`
- `boolean`
- `password`

支持的结构类型：

- `array`
- `object`

`enum` keyword 支持用于标量字段。换成检查器里的说法，就是 enum is supported as a keyword on scalar fields，例如：

```json
{ "type": "string", "enum": ["strict", "balanced"] }
```

界面会把标量 enum 字段渲染为 select control，把 `password` 字段渲染为 password input。

vNext does not provide host-managed secret storage for community plugin config。已保存的配置值仍然是普通插件配置值，可能出现在后端详情 payload 中。后端会在持久化前校验配置；前端校验只是便利层，不能作为唯一信任来源。

## UI 元数据

宿主会把 `configSchema` 渲染为低代码设置面板。优先使用标准 JSON Schema 展示字段：

- `title`：展示给用户看的字段名。
- `description`：标题下方的辅助说明。
- `default`：保存配置缺省该字段时使用的值。
- `enum`：允许值列表。
- `required`：object 中必填的属性。

AIO Coding Hub 还支持 vendor extension `x-aio-ui` 来表达界面展示提示。这些提示不改变后端校验语义。

根级 `x-aio-ui` 支持字段：

- `sections`：按顺序排列的字段分组。

字段级 `x-aio-ui` 支持字段：

- `section`：所属 section id。
- `order`：在 section 内的数字排序。
- `widget`：`text`、`textarea`、`password`、`number`、`switch`、`select`、`checkboxGroup` 或 `json`。
- `placeholder`：文本类输入框的占位文案。
- `warning`：常驻展示的警告文案。
- `warningWhenPartial`：checkbox group 被部分选择时展示的警告文案。
- `enumLabels`：把 enum 值映射为用户可读标签。
- `enumDescriptions`：把 enum 值映射为辅助说明。

当 widget hint 与字段类型不匹配时，宿主可以忽略该提示。例如 `checkboxGroup` 只适用于存在 `items.enum` 的 `array` 字段。

## 示例

```json
{
  "type": "object",
  "required": ["redactBeforeUpstream", "redactLogs", "profile"],
  "x-aio-ui": {
    "sections": [
      {
        "id": "routing",
        "title": "处理位置",
        "description": "选择插件在哪些阶段生效。",
        "order": 10
      },
      {
        "id": "content",
        "title": "要保护的内容",
        "description": "选择需要自动替换的敏感信息类型。",
        "order": 20
      }
    ]
  },
  "properties": {
    "redactBeforeUpstream": {
      "type": "boolean",
      "title": "发送给模型前处理",
      "description": "在请求离开本机前替换你选择的敏感信息。",
      "default": true,
      "x-aio-ui": {
        "section": "routing",
        "widget": "switch",
        "order": 10
      }
    },
    "profile": {
      "type": "string",
      "title": "保护强度",
      "default": "balanced",
      "enum": ["balanced", "strict"],
      "x-aio-ui": {
        "section": "routing",
        "widget": "select",
        "order": 20,
        "enumLabels": {
          "balanced": "平衡",
          "strict": "严格"
        }
      }
    },
    "sensitiveTypes": {
      "type": "array",
      "title": "要保护的内容",
      "description": "关闭某一项后，这类内容不会被该插件处理。",
      "default": ["email", "cn_phone"],
      "items": {
        "type": "string",
        "enum": ["email", "cn_phone"],
        "x-aio-ui": {
          "enumLabels": {
            "email": "邮箱地址",
            "cn_phone": "中国手机号"
          },
          "enumDescriptions": {
            "email": "例如 name@example.com。",
            "cn_phone": "例如 13344441520。"
          }
        }
      },
      "x-aio-ui": {
        "section": "content",
        "widget": "checkboxGroup",
        "order": 10,
        "warningWhenPartial": "关闭后，这类内容会原样发送给模型，也可能出现在本地日志中。"
      }
    }
  }
}
```
