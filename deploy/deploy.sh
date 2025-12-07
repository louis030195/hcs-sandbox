#!/bin/bash
# Deploy hyperv-kube to Azure
# Usage: ./deploy.sh [resource-group] [location] [password]

set -e

RG="${1:-hyperv-kube-rg}"
LOCATION="${2:-eastus}"
PASSWORD="${3:-}"
VM_NAME="hyperv-kube"

if [ -z "$PASSWORD" ]; then
  echo "Usage: ./deploy.sh <resource-group> <location> <password>"
  echo "  Example: ./deploy.sh hyperv-kube-rg eastus 'MyP@ssw0rd123!'"
  exit 1
fi

# Get current IP for NSG
MY_IP=$(curl -s ifconfig.me)
echo "Your IP: $MY_IP"

# Create resource group
echo "Creating resource group: $RG in $LOCATION"
az group create --name "$RG" --location "$LOCATION" --output none

# Deploy ARM template
echo "Deploying VM with Hyper-V (takes ~5 min)..."
DEPLOY_OUTPUT=$(az deployment group create \
  --resource-group "$RG" \
  --template-file "$(dirname "$0")/azuredeploy.json" \
  --parameters \
    vmName="$VM_NAME" \
    adminPassword="$PASSWORD" \
    allowedIP="$MY_IP" \
  --query "properties.outputs" \
  --output json)

PUBLIC_IP=$(echo "$DEPLOY_OUTPUT" | jq -r '.publicIP.value')
FQDN=$(echo "$DEPLOY_OUTPUT" | jq -r '.fqdn.value')

echo ""
echo "=== Deployment Complete ==="
echo "VM is rebooting to enable Hyper-V..."
echo ""
echo "Public IP: $PUBLIC_IP"
echo "FQDN: $FQDN"
echo "RDP: mstsc /v:$PUBLIC_IP"
echo "API: http://$PUBLIC_IP:8080 (after setup)"
echo ""
echo "=== Next Steps ==="
echo "1. Wait 2-3 min for reboot"
echo "2. RDP: mstsc /v:$PUBLIC_IP (user: hvadmin)"
echo "3. Download template VHDX and hvkube.exe"
echo "4. Run:"
echo "   hvkube template register --name win11 --vhdx C:\\HyperVKube\\Templates\\win11.vhdx"
echo "   hvkube pool create --name agents --template win11 --count 3"
echo "   hvkube pool provision agents --count 3"
echo "   hvkube pool prepare agents"
echo "   hvkube serve --port 8080"
