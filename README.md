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

## 安裝方式 (Installation)

你可以透過以下幾種方式安裝此工具：

### 1. 快速安裝腳本 (推薦)

這適合所有系統 (Mac/Ubuntu)，腳本會自動檢查並安裝所需的 Rust 環境。

```bash
./install.sh
```

### 2. 從 Crates.io 安裝

如果你已經安裝了 Rust，可以直接從官方套件庫下載安裝：

```bash
cargo install dlt2log
```

安裝後，即可在終端機直接執行 `dlt2log`指令。

---

## 開發者指南 (Developer Guide)

如果你想要修改原始碼或從本地編譯，請參考以下步驟：

### 快速編譯

```bash
./build.sh
```

### 常用指令 (Makefile)

*   `make build`: 編譯 Release 版本。
*   `make verify`: 自動轉換 `dlt_v1_v2/` 下的所有測試檔並驗證。
*   `make clean`: 清除暫存檔與生成的 log。

## 輸出格式

工具會將日誌轉換為以下格式：

```text
[Timestamp][AppID CtxID][LogLevel] Payload
```

例如：
```text
[16234747684009687][DMgr vdut][INFO] Create RootSwc singleton [root_swc.cpp:82]
```
