# dlt2log

這是一個用 Rust 寫的命令行工具，用於將汽車診斷日誌 (DLT) 檔轉換為人類可讀的文本格式。

## 功能特點

*   **支援雙版本**: 兼容 DLT v1 和 DLT v2 協議。
*   **強大的解析**:
    *   自動偵測並正確解析 Storage Header。
    *   支援 DLT v2 的可變長度 ID (ECU/App/Context ID)。
    *   **智慧過濾**: 自動移除 DLT v2 verbose 訊息中的二進位殘留，確保輸出乾淨。
*   **高效能**: 批次處理大量日誌訊息不掉幀。

## 使用方式

需要安裝 [Rust](https://www.rust-lang.org/tools/install)。

### 基本轉換

```bash
cargo run -- <input_file.dlt> -o <output_file.log>
```

### 範例

轉換專案內的範例檔案：

```bash
cargo run -- dlt_v1_v2/log_166.dlt -o log_166.log
```

## 輸出格式

工具會將日誌轉換為以下格式：

```text
[Timestamp][AppID CtxID][LogLevel] Payload
```

例如：
```text
[16234747684009687][DMgr vdut][INFO] Create RootSwc singleton [root_swc.cpp:82]
```
