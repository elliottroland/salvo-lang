package salvo.core.compare


fun mixHash(seed: Long, value: Long): Long {
    return seed * 31L + value
}
