#!/bin/bash

# ✅ Allow choosing the circuit via CLI argument (default to "liquidityverifier")
CIRCUIT=${1:-liquidityverifier}

# ✅ Automatically derive contract name by capitalizing the first letter
CONTRACT_NAME="$(tr '[:lower:]' '[:upper:]' <<< ${CIRCUIT:0:1})${CIRCUIT:1}"

# ✅ Set default PTAU file number
PTAU=${2:-14}  # If not provided, default to 14

# ✅ Determine input file dynamically based on CIRCUIT
INPUT_FILE="${CIRCUIT}_input.json"

# ✅ Define specific proof and public JSON files for each circuit
PROOF_FILE="${CIRCUIT}_proof.json"
PUBLIC_FILE="${CIRCUIT}_public.json"

# ✅ Ensure the input file exists before proceeding
if [ ! -f "$INPUT_FILE" ]; then
    echo "❌ Error: Input file '$INPUT_FILE' not found!"
    exit 1
fi

echo "🔹 Using input file: $INPUT_FILE"

# ✅ Ensure the ptau directory exists
mkdir -p ./ptau

# ✅ Check if the necessary ptau file exists, otherwise download it
if [ ! -f ./ptau/powersOfTau28_hez_final_${PTAU}.ptau ]; then    
    echo "----- Downloading powersOfTau28_hez_final_${PTAU}.ptau -----"
    wget -P ./ptau https://hermez.s3-eu-west-1.amazonaws.com/powersOfTau28_hez_final_${PTAU}.ptau || {
        echo "----- Error downloading powersOfTau28_hez_final_${PTAU}.ptau -----"
        exit 1
    }
else
   echo "----- powersOfTau28_hez_final_${PTAU}.ptau already exists -----"
fi

# ✅ Prepare Phase 2 of the trusted setup
echo "🔹 Preparing Phase 2..."
snarkjs powersoftau prepare phase2 ./ptau/powersOfTau28_hez_final_${PTAU}.ptau  -v
echo "✅ Phase 2 completed: ./ptau/powersOfTau28_hez_final_${PTAU}.ptau"

# ✅ Compile the chosen circuit
echo "🔹 Compiling circuit: ${CIRCUIT}.circom"
circom ${CIRCUIT}.circom --r1cs --wasm --sym --c
echo "✅ Circuit compiled: ${CIRCUIT}.r1cs"

# ✅ Generate Groth16 proving key
echo "🔹 Running Groth16 setup..."
snarkjs groth16 setup ${CIRCUIT}.r1cs ptau/powersOfTau28_hez_final_${PTAU}.ptau ${CIRCUIT}_0000.zkey
echo "✅ Groth16 setup completed: ${CIRCUIT}_0000.zkey"

# ✅ Contribute to the setup
snarkjs zkey contribute ${CIRCUIT}_0000.zkey ${CIRCUIT}_final.zkey --name="First contribution" -v -e="random" --overwrite

# ✅ Export the verification key
echo "🔹 Exporting verification key..."
snarkjs zkey export verificationkey ${CIRCUIT}_final.zkey ${CIRCUIT}_key.json
echo "✅ Verification key exported: ${CIRCUIT}_key.json"

# ✅ Generate witness dynamically based on selected circuit
echo "🔹 Generating witness..."
node ${CIRCUIT}_js/generate_witness.js ${CIRCUIT}_js/${CIRCUIT}.wasm $INPUT_FILE ${CIRCUIT}_js/witness.wtns
echo "✅ Witness generated"

# ✅ Generate zk-SNARK proof
echo "🔹 Generating zk-SNARK proof..."
snarkjs groth16 prove ${CIRCUIT}_final.zkey ${CIRCUIT}_js/witness.wtns $PROOF_FILE $PUBLIC_FILE
echo "✅ zk-SNARK proof generated: $PROOF_FILE"

# ✅ Verify the proof
echo "🔹 Verifying zk-SNARK proof..."
snarkjs groth16 verify ${CIRCUIT}_key.json $PUBLIC_FILE $PROOF_FILE
echo "✅ zk-SNARK proof verified"

# ✅ Export the Solidity verifier
echo "🔹 Exporting Solidity verifier..."
snarkjs zkey export solidityverifier ${CIRCUIT}_final.zkey Verifier.sol

# ✅ Update Solidity version in the verifier
sed -i.bak 's/pragma solidity >=0.7.0 <0.9.0;/pragma solidity ^0.8.24;/g' Verifier.sol

# ✅ Rename contract inside Solidity file
sed -i.bak "s/contract Groth16Verifier/contract ${CONTRACT_NAME}/g" Verifier.sol

# ✅ Rename Solidity file to match the contract name
mv Verifier.sol ${CONTRACT_NAME}.sol
echo "✅ Solidity file renamed to ${CONTRACT_NAME}.sol"

# ✅ Generate and print parameters of call to the verifier
if [[ "$CIRCUIT" == "liquidityverifier" ]]; then
    echo "🔹 Generating parameters for calling the verifier..."
    snarkjs generatecall | tee liquidityverifier_params.txt
    echo "✅ Parameters generated"
elif [[ "$CIRCUIT" == "swapverifier" ]]; then
    echo "🔹 Generating parameters for calling the verifier..."
    snarkjs generatecall | tee swapverifier_params.txt
    echo "✅ Parameters generated"
else
    echo "🔹 Generating parameters for $CIRCUIT..."
    snarkjs generatecall | tee ${CIRCUIT}_params.txt
    echo "✅ Parameters generated"
fi