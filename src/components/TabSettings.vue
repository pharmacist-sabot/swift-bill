<script setup lang="ts">
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { Settings2, Table2, Save, Plug, CheckCircle, XCircle, Loader2 } from 'lucide-vue-next'

interface DbConfig {
    host: string;
    port: number;
    database: string;
    username: string;
    password: string;
}

const props = defineProps<{ dbConfig: DbConfig }>();
const emit = defineEmits<{
    (e: "update:dbConfig", val: DbConfig): void;
    (e: "save"): void;
    (e: "connectionStatus", connected: boolean): void;
}>();

const status = ref<"idle" | "testing" | "success" | "error">("idle");
const message = ref("");
const saveStatus = ref<"idle" | "saved">("idle");

const isValid = computed(
    () =>
        props.dbConfig.host.trim() !== "" &&
        props.dbConfig.port > 0 &&
        props.dbConfig.database.trim() !== "" &&
        props.dbConfig.username.trim() !== "" &&
        props.dbConfig.password.trim() !== ""
);

const statusClass = computed(() => ({
    "status-success": status.value === "success",
    "status-error": status.value === "error",
    "status-info": status.value === "testing",
}));

const statusIcon = computed(() => {
    if (status.value === "success") return CheckCircle
    if (status.value === "error") return XCircle
    return Loader2
})

function update(field: keyof DbConfig, value: string | number) {
    emit("update:dbConfig", { ...props.dbConfig, [field]: value });
}

function saveConfig() {
    emit("save");
    saveStatus.value = "saved";
    setTimeout(() => { saveStatus.value = "idle"; }, 2500);
}

async function testConnection() {
    if (!isValid.value) {
        message.value = "กรุณากรอกข้อมูลให้ครบถ้วน";
        status.value = "error";
        return;
    }
    status.value = "testing";
    message.value = "กำลังทดสอบการเชื่อมต่อ...";
    try {
        const msg = await invoke<string>("test_connection", {
            config: { ...props.dbConfig },
        });
        status.value = "success";
        message.value = msg;
        emit("connectionStatus", true);
    } catch (e) {
        status.value = "error";
        message.value = String(e);
        emit("connectionStatus", false);
    }
}
</script>

<template>
<div class="settings-wrap">
    <div class="card">
        <div class="card-title">
            <Settings2 :size="17" /> ตั้งค่าการเชื่อมต่อฐานข้อมูล
        </div>
        <div class="card-desc">เชื่อมต่อ SQL Server (INVS) ผ่าน TDS Protocol โดยตรง — ไม่ต้องติดตั้ง ODBC Driver</div>

        <div class="form-grid">
            <div class="form-group">
                <label>Host / IP Address</label>
                <input type="text" :value="dbConfig.host"
                    @input="update('host', ($event.target as HTMLInputElement).value)" placeholder="192.168.1.100"
                    autocapitalize="none" autocorrect="off" spellcheck="false" />
            </div>
            <div class="form-group">
                <label>Port</label>
                <input type="number" :value="dbConfig.port"
                    @input="update('port', parseInt(($event.target as HTMLInputElement).value) || 1433)"
                    placeholder="1433" />
            </div>
            <div class="form-group">
                <label>Database Name</label>
                <input type="text" :value="dbConfig.database"
                    @input="update('database', ($event.target as HTMLInputElement).value)" placeholder="INVS"
                    autocapitalize="none" autocorrect="off" spellcheck="false" />
            </div>
            <div class="form-group">
                <label>Username</label>
                <input type="text" :value="dbConfig.username"
                    @input="update('username', ($event.target as HTMLInputElement).value)" placeholder="sa"
                    autocapitalize="none" autocorrect="off" spellcheck="false" />
            </div>
            <div class="form-group">
                <label>Password</label>
                <input type="password" :value="dbConfig.password"
                    @input="update('password', ($event.target as HTMLInputElement).value)" placeholder="••••••••" />
            </div>
        </div>

        <div class="actions" style="display:flex; gap:12px; flex-wrap:wrap;">
            <button class="btn btn-success btn-lg" :disabled="!isValid" @click="saveConfig">
                <CheckCircle v-if="saveStatus === 'saved'" :size="16" />
                <Save v-else :size="16" />
                {{ saveStatus === 'saved' ? 'บันทึกแล้ว!' : 'บันทึกการตั้งค่า' }}
            </button>
            <button class="btn btn-primary btn-lg" :disabled="status === 'testing' || !isValid" @click="testConnection">
                <span v-if="status === 'testing'" class="spinner"></span>
                <Plug v-else :size="16" />
                {{ status === "testing" ? "กำลังทดสอบ..." : "ทดสอบการเชื่อมต่อ" }}
            </button>
        </div>

        <div v-if="message" :class="['status-msg', statusClass]" style="margin-top:12px">
            <component :is="statusIcon" :size="15" />
            {{ message }}
        </div>


    </div>

    <div class="card">
        <div class="card-title">
            <Table2 :size="17" /> ข้อมูลที่ระบบดึงจาก INVS
        </div>
        <table class="data-table">
            <thead>
                <tr>
                    <th>ตาราง</th>
                    <th>คอลัมน์ที่ใช้</th>
                </tr>
            </thead>
            <tbody>
                <tr>
                    <td><code>MS_IVO</code></td>
                    <td><code>INVOICE_NO, VENDOR_CODE, TOTAL_COST, RECEIVE_DATE</code></td>
                </tr>
                <tr>
                    <td><code>COMPANY</code></td>
                    <td><code>COMPANY_CODE, COMPANY_NAME, KEY_WORD</code></td>
                </tr>
            </tbody>
        </table>
    </div>
</div>
</template>

<style scoped>
.settings-wrap {
    width: 100%;
}

.actions {
    margin-top: 22px;
}

code {
    font-family: "Consolas", "Fira Code", monospace;
    font-size: 12px;
    background: var(--c-primary-light);
    padding: 2px 6px;
    border-radius: 4px;
    color: var(--c-primary);
    border: 1px solid #F0C4B8;
}

@media (prefers-color-scheme: dark) {
    code {
        background: #2A0808;
        color: var(--c-primary);
        border-color: #501515;
    }
}
</style>
