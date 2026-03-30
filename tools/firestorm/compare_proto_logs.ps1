param(
  [Parameter(Mandatory=$true)][string]$FirestormLog,
  [Parameter(Mandatory=$true)][string]$ViewerLog
)

if (-not (Test-Path $FirestormLog)) { throw "Firestorm log not found: $FirestormLog" }
if (-not (Test-Path $ViewerLog)) { throw "Viewer log not found: $ViewerLog" }

$fs = Get-Content $FirestormLog -Raw
$vv = Get-Content $ViewerLog -Raw

function Count-Match([string]$text, [string]$pattern) {
  return ([regex]::Matches($text, $pattern, [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)).Count
}

$rows = @()
$rows += [pscustomobject]@{ Signal='firestorm:enable_simulator'; Count=Count-Match $fs 'AGVProto.*enable_simulator' }
$rows += [pscustomobject]@{ Signal='firestorm:crossed_region'; Count=Count-Match $fs 'AGVProto.*crossed_region' }
$rows += [pscustomobject]@{ Signal='firestorm:agent_movement_complete'; Count=Count-Match $fs 'AGVProto.*agent_movement_complete' }
$rows += [pscustomobject]@{ Signal='firestorm:start_ping_check'; Count=Count-Match $fs 'AGVProto.*start_ping_check' }
$rows += [pscustomobject]@{ Signal='firestorm:complete_ping_check'; Count=Count-Match $fs 'AGVProto.*complete_ping_check' }
$rows += [pscustomobject]@{ Signal='firestorm:agent_throttle_send'; Count=Count-Match $fs 'AGVProto.*agent_throttle_send' }
$rows += [pscustomobject]@{ Signal='firestorm:agent_update_send'; Count=Count-Match $fs 'AGVProto.*agent_update_send' }
$rows += [pscustomobject]@{ Signal='firestorm:recv_object_update_full'; Count=Count-Match $fs 'AGVProto.*recv_object_update kind=full' }
$rows += [pscustomobject]@{ Signal='firestorm:recv_object_update_compressed'; Count=Count-Match $fs 'AGVProto.*recv_object_update kind=compressed' }
$rows += [pscustomobject]@{ Signal='firestorm:recv_object_update_cached'; Count=Count-Match $fs 'AGVProto.*recv_object_update kind=cached' }
$rows += [pscustomobject]@{ Signal='firestorm:recv_object_update_terse'; Count=Count-Match $fs 'AGVProto.*recv_object_update kind=terse' }

$rows += [pscustomobject]@{ Signal='viewer:enable_simulator_eventq'; Count=Count-Match $vv 'EnableSimulator observed in EventQueueGet payload' }
$rows += [pscustomobject]@{ Signal='viewer:retargeting'; Count=Count-Match $vv 'retargeting social circuit to' }
$rows += [pscustomobject]@{ Signal='viewer:object_updates_nonzero_lines'; Count=Count-Match $vv 'object_updates=[1-9][0-9]*' }
$rows += [pscustomobject]@{ Signal='viewer:start_ping_check_nonzero_lines'; Count=Count-Match $vv 'start_ping_check=[1-9][0-9]*' }
$rows += [pscustomobject]@{ Signal='viewer:layer_data_nonzero_lines'; Count=Count-Match $vv 'layer_data=[1-9][0-9]*' }

$rows | Format-Table -AutoSize

Write-Host "`nQuick verdict:" -ForegroundColor Cyan
$fsObj = ($rows | Where-Object Signal -like 'firestorm:recv_object_update*' | Measure-Object Count -Sum).Sum
$vvObj = ($rows | Where-Object Signal -eq 'viewer:object_updates_nonzero_lines').Count
if ($fsObj -gt 0 -and $vvObj -eq 0) {
  Write-Host "Firestorm receives object updates while viewer reports none -> protocol gap confirmed." -ForegroundColor Yellow
} elseif ($fsObj -eq 0) {
  Write-Host "Firestorm log did not show object updates in this capture window; re-run in a busier scene." -ForegroundColor Yellow
} else {
  Write-Host "Both logs show object update activity; inspect UUID/decode/render stages next." -ForegroundColor Green
}
