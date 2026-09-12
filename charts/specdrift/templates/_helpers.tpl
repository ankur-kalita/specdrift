{{/*
Chart name, truncated to 63 chars because that is the Kubernetes label limit.
*/}}
{{- define "specdrift.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Fully qualified name. Argo CD sets the release name from the Application,
so this is what actually distinguishes two installs in one namespace.
*/}}
{{- define "specdrift.fullname" -}}
{{- if contains .Release.Name .Chart.Name -}}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else -}}
{{- printf "%s-%s" .Release.Name .Chart.Name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}

{{- define "specdrift.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{- define "specdrift.labels" -}}
helm.sh/chart: {{ include "specdrift.chart" . }}
{{ include "specdrift.selectorLabels" . }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{- define "specdrift.selectorLabels" -}}
app.kubernetes.io/name: {{ include "specdrift.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{- define "specdrift.serviceAccountName" -}}
{{- if .Values.serviceAccount.create -}}
{{- default (include "specdrift.fullname" .) .Values.serviceAccount.name }}
{{- else -}}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
The shell command specdrift runs, derived from .Values.mode.

snapshot: write facts to a writable temp dir and print them.
diff:     compare the live machine against the mounted baseline.

Note readOnlyRootFilesystem is true, so output must go to the emptyDir
mounted at /work — not /tmp.
*/}}
{{- define "specdrift.command" -}}
{{- if eq .Values.mode "diff" -}}
specdrift diff /baseline/baseline.json{{ if .Values.includeVolatile }} --all{{ end }}
{{- else -}}
specdrift snapshot -o /work/snapshot.json && cat /work/snapshot.json
{{- end }}
{{- end }}
