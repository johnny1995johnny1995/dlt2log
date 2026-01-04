# 驗證步驟：第一階段 (DLT v1/v2 解析)

請使用本指南驗證 `dlt2log` 工具是否能正確處理 DLT v1 和 v2 檔案，並產生乾淨且完整的 log。

## 1. 執行批次處理
在終端機中，從專案根目錄 (`./dlt2log`) 執行以下指令以處理所有範例檔案：

```bash
for f in dlt_v1_v2/*.dlt; do 
    echo "Processing $f..."
    cargo run -- "$f" -o "${f%.dlt}.log"
done
```

## 2. 預期輸出 (應該看到的結果)

### 在終端機中
你應該會看到每個檔案都顯示成功訊息，且訊息數量相當多：

*   **`log_163.dlt`**: `Successfully processed 142702 messages.`
*   **`log_164.dlt`**: `Successfully processed 143479 messages.`
*   **`log_165.dlt`**: `Successfully processed 129873 messages.`
*   **`log_166.dlt`**: `Successfully processed 67262 messages.` (DLT v2)
*   **`log_167.dlt`**: `Successfully processed 1895 messages.`
*   **`log_168.dlt`**: `Successfully processed 1571 messages.`

### 在 Log 檔案中
打開任何產生的 `.log` 檔案 (例如 `dlt_v1_v2/log_166.log`)。你應該看到：
1.  **格式**: `[Timestamp][AppID CtxID][LogLevel] Payload`
2.  **乾淨的文字**: Payload 應該是人類可讀的英文文字。
3.  **無二進位碼**: 訊息開頭不應出現奇怪的符號或 Unicode 替換字元。

**正確輸出範例：**
```text
[16234747684009687][DMgr vdut][INFO] Create RootSwc singleton [root_swc.cpp:82]
```

## 3. 非預期輸出 (不應該看到的結果)

### ❌ "Successfully processed 1 messages"
如果你在 `log_163`、`164` 或 `165` 看到此訊息，表示 **DLT v1 解析器壞了** (可能將整個檔案當作一則訊息處理)。
*   *原因*: `dlt-core` reader 的緩衝問題。
*   *狀態*: **已修復** (透過手動 frame 提取)。

### ❌ 亂碼 / 二進位字元
如果 log 看起來像這樣：
`[...][INFO] \u{0}\u{1}Start logging...`
這表示 **Payload 提取壞了** (將二進位的 Type Info 欄位誤解為文字)。
*   *原因*: DLT v2 verbose payloads 包含二進位標頭。
*   *狀態*: **已修復** (透過啟發式 ASCII 過濾)。

### ❌ "Error parsing" 或 負數時間戳
如果你看到巨大的負數時間戳或解析器崩潰錯誤。
*   *原因*: Endianness 不匹配或標頭偏移量錯誤。
*   *狀態*: **已修復**。
