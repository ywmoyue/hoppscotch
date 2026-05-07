<template>
  <div class="flex h-full flex-col gap-4 p-4">
    <div v-if="!isDesktop" class="text-secondaryLight">
      gRPC is only available in the desktop app.
    </div>

    <template v-else>
      <div class="grid gap-4 md:grid-cols-2">
        <label class="flex flex-col gap-2">
          <span class="text-secondaryLight">Proto file</span>
          <input type="file" accept=".proto" @change="onProtoFileChange" />
        </label>
        <label class="flex flex-col gap-2">
          <span class="text-secondaryLight">Server address</span>
          <input
            v-model="serverAddress"
            type="text"
            class="rounded border border-divider bg-primary p-2"
            placeholder="127.0.0.1:50051"
          />
        </label>
      </div>

      <label class="flex flex-col gap-2">
        <span class="text-secondaryLight">Method</span>
        <select
          v-model="selectedMethodId"
          class="rounded border border-divider bg-primary p-2"
        >
          <option disabled value="">Select a method</option>
          <option v-for="method in methods" :key="method.id" :value="method.id">
            {{ method.label }}
          </option>
        </select>
      </label>

      <label class="flex flex-col gap-2">
        <span class="text-secondaryLight">Request JSON</span>
        <textarea
          v-model="requestJSON"
          class="min-h-40 rounded border border-divider bg-primary p-2 font-mono"
          spellcheck="false"
        />
      </label>

      <div class="flex gap-2">
        <HoppButtonPrimary
          label="Send gRPC Request"
          :disabled="isSending"
          @click="sendRequest"
        />
      </div>

      <div
        v-if="errorMessage"
        class="rounded border border-red-500/40 p-3 text-red-400"
      >
        {{ errorMessage }}
      </div>

      <label class="flex flex-col gap-2">
        <span class="text-secondaryLight">Response JSON</span>
        <textarea
          :value="responseJSON"
          class="min-h-48 rounded border border-divider bg-primary p-2 font-mono"
          readonly
        />
      </label>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue"
import { invoke } from "@tauri-apps/api/core"
import { getKernelMode } from "@hoppscotch/kernel"
import * as protobuf from "protobufjs"

type MethodEntry = {
  id: string
  label: string
  methodPath: string
  requestType: protobuf.Type
  responseType: protobuf.Type
}

const isDesktop = getKernelMode() === "desktop"
const serverAddress = ref("")
const selectedMethodId = ref("")
const requestJSON = ref("{}")
const responseJSON = ref("")
const errorMessage = ref("")
const isSending = ref(false)
const methods = ref<MethodEntry[]>([])

const selectedMethod = computed(() =>
  methods.value.find((method) => method.id === selectedMethodId.value)
)

const uint8ArrayToBase64 = (bytes: Uint8Array) => {
  const chunkSize = 0x8000
  const chunks: string[] = []

  for (let index = 0; index < bytes.length; index += chunkSize) {
    const chunk = bytes.subarray(index, index + chunkSize)
    chunks.push(String.fromCharCode(...chunk))
  }

  return btoa(chunks.join(""))
}

const base64ToUint8Array = (value: string) => {
  const binary = atob(value)
  return Uint8Array.from(binary, (char) => char.charCodeAt(0))
}

const getMethodEntries = (root: protobuf.Root) => {
  const entries: MethodEntry[] = []

  const walk = (namespace: protobuf.NamespaceBase) => {
    for (const nested of namespace.nestedArray ?? []) {
      if (nested instanceof protobuf.Service) {
        const serviceName = nested.fullName.replace(/^\./, "")

        for (const method of Object.values(nested.methods)) {
          if (!method.resolvedRequestType || !method.resolvedResponseType)
            continue

          entries.push({
            id: `${serviceName}.${method.name}`,
            label: `${serviceName}.${method.name}`,
            methodPath: `${serviceName}/${method.name}`,
            requestType: method.resolvedRequestType,
            responseType: method.resolvedResponseType,
          })
        }
      }

      if (nested instanceof protobuf.Namespace) {
        walk(nested)
      }
    }
  }

  walk(root)
  return entries
}

const onProtoFileChange = async (event: Event) => {
  errorMessage.value = ""
  responseJSON.value = ""

  const target = event.target as HTMLInputElement
  const file = target.files?.[0]
  if (!file) return

  try {
    const protoContent = await file.text()
    const parsed = protobuf.parse(protoContent, {
      keepCase: true,
    })
    parsed.root.resolveAll()

    methods.value = getMethodEntries(parsed.root)
    selectedMethodId.value = methods.value[0]?.id ?? ""
  } catch (error) {
    methods.value = []
    selectedMethodId.value = ""
    errorMessage.value = `Failed to parse proto file: ${String(error)}`
  }
}

const sendRequest = async () => {
  errorMessage.value = ""
  responseJSON.value = ""

  if (!selectedMethod.value) {
    errorMessage.value = "Please select a gRPC method."
    return
  }

  if (!serverAddress.value.trim()) {
    errorMessage.value = "Please enter the gRPC server address."
    return
  }

  try {
    isSending.value = true

    const requestObject = JSON.parse(requestJSON.value || "{}")
    const requestMessage =
      selectedMethod.value.requestType.fromObject(requestObject)
    const payload = selectedMethod.value.requestType
      .encode(requestMessage)
      .finish()

    const responseBase64 = await invoke<string>("grpc_unary_invoke", {
      serverAddress: serverAddress.value.trim(),
      methodPath: selectedMethod.value.methodPath,
      payloadBase64: uint8ArrayToBase64(payload),
    })

    const responsePayload = base64ToUint8Array(responseBase64)
    const decodedResponse =
      selectedMethod.value.responseType.decode(responsePayload)
    const responseObject = selectedMethod.value.responseType.toObject(
      decodedResponse,
      {
        longs: String,
        enums: String,
        bytes: String,
        defaults: true,
      }
    )

    responseJSON.value = JSON.stringify(responseObject, null, 2)
  } catch (error) {
    errorMessage.value = `gRPC request failed: ${String(error)}`
  } finally {
    isSending.value = false
  }
}
</script>
