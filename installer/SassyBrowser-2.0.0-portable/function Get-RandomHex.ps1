function Get-RandomHex {
    param(
        [int]$Length = 32 # Length in characters (32 chars = 16 bytes)
    )
    # Convert character length to byte length (each byte is 2 hex chars)
    $byteLength = [Math]::Ceiling($Length / 2)

    # Create a cryptographically secure random number generator
    $rand = [System.Security.Cryptography.RandomNumberGenerator]::Create()

    # Create a byte array of the required length
    $bytes = [byte[]]::new($byteLength)

    # Fill the array with random bytes
    $rand.GetBytes($bytes)

    # Convert the byte array to a hexadecimal string and return the specified length
    $hexString = [System.Convert]::ToHexString($bytes)
    return $hexString.Substring(0, $Length)
}

# Example: Generate a 32-character (256-bit) random hex string
Get-RandomHex -Length 64
wait
