function Convert-ToPublicSupervisionReviewBody {
    param([object]$Review)
    # Old and new review routes share this strict DTO. The node injects identity.
    return [ordered]@{
        verdict = [string](Get-ObjectField $Review 'verdict')
        summary = [string](Get-ObjectField $Review 'summary')
        improvements = @((Get-ObjectField $Review 'improvements'))
    }
}
