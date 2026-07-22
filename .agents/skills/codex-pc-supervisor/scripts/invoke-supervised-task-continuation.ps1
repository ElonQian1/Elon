function New-ResumeTaskBody {
    param(
        [object]$ParentDetail,
        [string]$RequestedParentTaskId,
        [string[]]$BodyCriteria,
        [string]$BodyImprovementPolicy
    )
    Assert-SafeResumeParentDetail $ParentDetail $RequestedParentTaskId
    $parentRecord = Get-RecordFromDetail $ParentDetail
    $parentProjectId = [string](Get-ObjectField $parentRecord 'project_id')
    $parentWorkspace = [string](Get-ObjectField $parentRecord 'workspace_path')
    $rootTask = Get-RootTaskFromDetail $ParentDetail $RequestedParentTaskId
    # The node is the authority for root requirement, lineage, acceptance
    # criteria and workspace identity. Never copy the parent prompt here.
    $resumePrompt = "Resolve elon.resume_context.v1 for parent_task_id=$RequestedParentTaskId and root_task_id=$rootTask."
    return New-SupervisedTaskBody $parentProjectId $parentWorkspace $resumePrompt `
        'resume_original' $RequestedParentTaskId $rootTask $BodyCriteria $BodyImprovementPolicy
}

function New-SupersedeTaskBody {
    param(
        [object]$ParentDetail,
        [string]$RequestedParentTaskId,
        [string]$RevisedPrompt,
        [string[]]$BodyCriteria,
        [string]$Reason,
        [string]$BodyImprovementPolicy
    )
    Assert-SafeResumeParentDetail $ParentDetail $RequestedParentTaskId
    if ([string]::IsNullOrWhiteSpace($RevisedPrompt)) {
        throw 'Supersede requires the complete revised Prompt.'
    }
    if (@($BodyCriteria | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }).Count -eq 0) {
        throw 'Supersede requires explicit revised acceptance criteria.'
    }
    if ([string]::IsNullOrWhiteSpace($Reason)) {
        throw 'Supersede requires AmendmentReason.'
    }
    $parentRecord = Get-RecordFromDetail $ParentDetail
    $parentProjectId = [string](Get-ObjectField $parentRecord 'project_id')
    $parentWorkspace = [string](Get-ObjectField $parentRecord 'workspace_path')
    $workspaceStatus = Get-ObjectField $parentRecord 'workspace_status'
    $recordedBaseWorkspace = [string](Get-ObjectField $workspaceStatus 'base_workspace_path')
    if (-not [string]::IsNullOrWhiteSpace($recordedBaseWorkspace)) {
        $parentWorkspace = $recordedBaseWorkspace
    }
    $rootTask = Get-RootTaskFromDetail $ParentDetail $RequestedParentTaskId
    $body = New-SupervisedTaskBody $parentProjectId $parentWorkspace $RevisedPrompt `
        'resume_original' $RequestedParentTaskId $rootTask $BodyCriteria $BodyImprovementPolicy
    $body['contract_revision'] = [ordered]@{
        schema = 'elon.supervision.contract_revision.v1'
        reason = $Reason.Trim()
    }
    return $body
}
