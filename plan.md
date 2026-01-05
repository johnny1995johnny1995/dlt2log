# dlt2log

## 簡介

**dlt2log** 是一個將 AUTOSAR DLT（Diagnostic Log and Trace）二進制檔案轉換為人類可讀文字 log 格式的工具。

### 什麼是 DLT？

DLT 是 AUTOSAR 標準中定義的診斷日誌與追蹤協議，主要用於嵌入式系統（特別是汽車電子）的日誌記錄。DLT 檔案以二進制格式儲存，包含：

- **Storage Header**: DLT 標記、時間戳（秒/微秒）、ECU ID
- **Standard Header**: Header 類型、訊息計數器、訊息長度
- **Extended Header**: 訊息資訊、引數數量、App ID、Context ID
- **Payload**: 實際的日誌內容

### 為什麼需要 dlt2log？

1. **可讀性**：二進制 DLT 檔案無法直接閱讀，需要轉換成文字格式才能進行日常分析。
2. **整合性**：轉換後的文字日誌可輕易與 `grep`、`awk`、`sed` 或現代化的日誌分析系統整合。
3. **格式統一**：確保 DLT v1 與 v2 不同來源的日誌，能以統一的時間戳與欄位格式呈現。

### DLT v1 vs v2 支援說明

根據對 `COVESA/dlt-viewer` 源碼的調查，DLT 協定主要分為兩個版本，其 Header 結構有顯著差異：
- **DLT v1**:
  - Header Type (HTYP) 為 1 位元組，開頭標記通常為 `0x35`。
  - 使用固定的 4 位元組 ECU ID, App ID 和 Context ID。
  - Storage Header 提供微秒 (ms) 級時間戳。
- **DLT v2 (AUTOSAR R20-11)**:
  - Header Type 2 (HTYP2) 為 **4 位元組 (uint32_t)**，開頭標記通常為 **`0x4c`**。
  - **變動長度識別碼**: ECU ID, App ID, Context ID 前面各帶 1 位元組的長度資訊。
  - **高精度時間戳**: `Dltv2StorageHeader` 提供 **納秒 (ns)** 級絕對時間戳 (`seconds` + `nanoseconds`)。

**本工具實作方案**：
1. **主要支援**: 優先確保 DLT v1 的正確轉換。
2. **協定相容**: 實作 `htyp2` 的解析邏輯，動態處理 v2 的變動長度 ID。
3. **時間對齊**: 無論來源是 v1 或 v2，輸出統一轉換為微秒格式（如範例所示：`[1767000361264126]`）。
4. **Storage Header**: 自動偵測 `DLT\x01` 標記並依據協定版本讀取對應的 Storage Header。

### 參考資料 (dlt-viewer 源碼)

為了精確解析 DLT v2，參考了以下 `COVESA/dlt-viewer` 的核心檔案：
- [dlt_common.h](https://github.com/COVESA/dlt-viewer/blob/master/qdlt/dlt_common.h): 定義了 `Dltv2header`, `Dltv2StorageHeader` 及 `htyp2` 位元遮罩。
- [dlt_common.c](https://github.com/COVESA/dlt-viewer/blob/master/qdlt/dlt_common.c): 實作了關鍵的解析邏輯（如 `dltv2_file_read_header_raw`），包含如何讀取變動長度 ID。
- [qdltmsg.cpp](https://github.com/COVESA/dlt-viewer/blob/master/qdlt/qdltmsg.cpp): 展示了 `QDltMsg::setMsg` 如何區別 v1 與 v2 訊息。

---

## 技術選型

使用 Rust 語言及 [dlt-core](https://crates.io/crates/dlt-core) 套件來實現：

### dlt-core 特性

- ✅ 符合官方 AUTOSAR DLT 規範
- ✅ 高效解析二進制 DLT 內容（~409 MB/s）
- ✅ 序列化 DLT 訊息
- ✅ 支援 non-verbose 訊息（透過 FIBEX 檔案）

### Feature Flags

| Feature | 說明 |
|---------|------|
| `statistics` | 提供 DLT 內容統計摘要 |
| `fibex` | 解析 non-verbose 訊息的 FIBEX 設定 |
| `debug` | 額外的 debug 輸出 |
| `serialization` | 提供 serde Serialize/Deserialize |
| `stream` | 支援從 stream 解析 DLT 訊息 |

---

## 做法

### 核心架構

```
┌─────────────────┐    ┌──────────────┐    ┌─────────────────┐
│  DLT Binary     │ -> │  dlt-core    │ -> │  Text Log       │
│  (.dlt files)   │    │  Parser      │    │  Output         │
└─────────────────┘    └──────────────┘    └─────────────────┘
```

### 實作步驟

1. **讀取 DLT 檔案**
   ```rust
   use dlt_core::read::{DltMessageReader, read_message};
   
   let dlt_file = File::open(&dlt_file_path)?;
   let mut dlt_reader = DltMessageReader::new(dlt_file, true);
   ```

2. **解析訊息**
   ```rust
   loop {
       match read_message(&mut dlt_reader, None) {
           Ok(Some(msg)) => {
               // 處理訊息
           }
           Ok(None) => break,
           Err(DltParseError::ParsingHickup(_)) => continue,
           Err(e) => return Err(e),
       }
   }
   ```

3. **格式化輸出**
   ```
   [1767000361264126][VExc main][INFO] {"id": 4, "desc": "Process expected to be running", "name": "TboxEventDumpProcess"}
   ```
   格式規則：`[timestamp][AppID ContextID][LogLevel] payload`
   - `timestamp`: 使用 DLT Storage Header 中的微秒時間戳 (Storage Header 中的 seconds * 10^6 + microseconds)。
   - `AppID`: 4 字元應用程式標識符。
   - `ContextID`: 4 字元上下文標識符。
   - `LogLevel`: 日誌層級 (INFO, WARN, ERROR, DEBUG 等)。
   - `payload`: 實際的日誌內容。

---

## 使用方式

### 安裝

```bash
# 從 source 編譯
cargo build --release

# 執行
./target/release/dlt2log <input.dlt> [output.log]
```

### 命令列參數

```bash
dlt2log [OPTIONS] <INPUT_FILE>

Arguments:
  <INPUT_FILE>    輸入的 DLT 檔案路徑

Options:
  -o, --output    輸出檔案路徑（預設為輸入檔案所在的同目錄，並以 .log 為副檔名）
  -v, --verbose   詳細輸出模式
  -h, --help      顯示說明
```

### 範例

```bash
# 基本轉換（輸出到與輸入檔案同目錄的同名 .log 檔案）
dlt2log log_163.dlt
# 執行後會產生 log_163.log

# 指定輸出檔案路徑
dlt2log log_163.dlt -o output.log

# 批次處理
for f in dlt_v1_v2/*.dlt; do
    dlt2log "$f" -o "${f%.dlt}.log"
done
```

---

## TODO

### Phase 1: 基礎功能 ✨

- [x] 專案初始化（Cargo.toml 與依賴設定）
- [x] 實作 DLT 檔案讀取器（支援 Storage Header 自動偵測）
- [x] 實作 DLT v1/v2 訊息解析（處理 HTYP2 與變動長度 ID）
- [x] 實作目標文字格式化（輸出絕對時間微秒戳記）
- [x] 命令列介面（支援輸入、自動預設輸出路徑與詳細模式）

### Phase 2: 發佈與維護 📦

- [x] 註冊 Crates.io 帳號並執行 `cargo publish`
- [ ] 建立 GitHub Release 與 CI/CD 自動化

### Phase 3: 進階功能 🚀

- [ ] 提供精確過濾（按 AppID, ContextID, LogLevel 過濾）
    - [ ] 新增 `--filter` CLI 參數
    - [ ] 實作過濾邏輯
    - [ ] 整合至 DLT 解析流程
- [ ] 增加時間範圍篩選功能
    - [ ] 新增 `--start` CLI 參數
    - [ ] 新增 `--end` CLI 參數
    - [ ] 實作時間範圍篩選邏輯
    - [ ] 整合至 DLT 解析流程

### Phase 4: 效能與優化 ⚡

- [ ] 支援多執行緒平行處理大檔案
    - [ ] 新增 `--threads` CLI 參數
    - [ ] 實作多執行緒處理邏輯
    - [ ] 整合至 DLT 解析流程
- [ ] 優化記憶體管理與串流處理
    - [ ] 新增 `--stream` CLI 參數
    - [ ] 實作串流處理邏輯
    - [ ] 整合至 DLT 解析流程
- [ ] 統計數據報表輸出 (ECU/AppID 佔比)
    - [ ] 新增 `--stats` CLI 參數
    - [ ] 實作統計數據報表輸出邏輯
    - [ ] 整合至 DLT 解析流程

---

## 現有測試資料

```
dlt_v1_v2/
├── log_163.dlt  (10.0 MB) [dlt v1]
├── log_164.dlt  (10.0 MB) [dlt v1]
├── log_165.dlt  (9.0 MB) [dlt v1]
├── log_166.dlt  (5.6 MB) [dlt v2]
├── log_167.dlt  (378 KB) [dlt v1]
└── log_168.dlt  (285 KB) [dlt v1]
```

---

## 參考資源

- [dlt-core on crates.io](https://crates.io/crates/dlt-core)
- [dlt-core documentation](https://docs.rs/dlt-core/latest/dlt_core/)
- [AUTOSAR DLT Specification](https://www.autosar.org/)
