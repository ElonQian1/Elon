#requires -Version 7.0
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'chatgpt-text-block-docx-evidence.ps1')
$fixture=Get-ChatGptTextBlockDocxFixture
$root=Join-Path (Split-Path -Parent $PSScriptRoot) ('.ai-tmp/docx-evidence-'+[Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root -Force|Out-Null
$word='http://schemas.openxmlformats.org/wordprocessingml/2006/main'
$passed=0
function Assert($Condition,[string]$Label){if(-not $Condition){throw "failed:$Label"};$script:passed++}
function Reject([scriptblock]$Action,[string]$Label){
    $rejected=$false
    try{& $Action|Out-Null}catch{$rejected=$true}
    Assert $rejected $Label
}
function Element($Parent,[string]$Name,[hashtable]$Attributes=@{}){
    $node=$Parent.OwnerDocument.CreateElement('w',$Name,$word)
    foreach($key in $Attributes.Keys){[void]$node.SetAttribute($key,$word,[string]$Attributes[$key])}
    [void]$Parent.AppendChild($node);return ,$node
}
function Run($Parent,[string]$Text,[string]$Style=''){
    $run=Element $Parent r
    if($Style){$properties=Element $run rPr;Element $properties $Style|Out-Null}
    foreach($part in [regex]::Split($Text,"([`n`t])")){
        if($part -ceq "`n"){Element $run br|Out-Null}
        elseif($part -ceq "`t"){Element $run tab|Out-Null}
        elseif($part){$textNode=Element $run t;$textNode.InnerText=$part}
    }
}
function PackageParts {
    $doc=[xml]"<w:document xmlns:w='$word'><w:body/></w:document>"
    $body=$doc.DocumentElement.FirstChild
    $table=$null;$row=$null
    for($i=0;$i -lt $fixture.paragraphs.Count;$i++){
        $target=$body
        if($i -ge 4 -and $i -le 7){
            if($i -eq 4){$table=Element $body tbl}
            if($i % 2 -eq 0){$row=Element $table tr}
            $target=Element $row tc
        }
        $p=Element $target p
        if($i -eq 0){$pr=Element $p pPr;Element $pr pStyle @{val='Heading1'}|Out-Null}
        if($i -in @(2,3)){
            $pr=Element $p pPr;$num=Element $pr numPr
            Element $num numId @{val='1'}|Out-Null;Element $num ilvl @{val='0'}|Out-Null
        }
        if($i -eq 1){
            $parts=$fixture.paragraphs[$i] -csplit 'Bold and italic'
            Run $p $parts[0];Run $p 'Bold' b;Run $p ' and ';Run $p 'italic' i;Run $p $parts[1]
        }else{Run $p $fixture.paragraphs[$i]}
    }
    $office='http://schemas.openxmlformats.org/officeDocument/2006/relationships/'
    $relations='http://schemas.openxmlformats.org/package/2006/relationships'
    $type='application/vnd.openxmlformats-officedocument.wordprocessingml.'
    [ordered]@{
        '[Content_Types].xml'="<Types xmlns='http://schemas.openxmlformats.org/package/2006/content-types'><Default Extension='xml' ContentType='application/xml'/><Default Extension='rels' ContentType='application/vnd.openxmlformats-package.relationships+xml'/><Override PartName='/word/document.xml' ContentType='${type}document.main+xml'/><Override PartName='/word/styles.xml' ContentType='${type}styles+xml'/><Override PartName='/word/numbering.xml' ContentType='${type}numbering+xml'/></Types>"
        '_rels/.rels'="<Relationships xmlns='$relations'><Relationship Id='r1' Type='${office}officeDocument' Target='word/document.xml'/></Relationships>"
        'word/document.xml'=$doc.OuterXml
        'word/styles.xml'="<w:styles xmlns:w='$word'><w:style w:type='paragraph' w:styleId='Normal'/><w:style w:type='paragraph' w:styleId='Heading1'/></w:styles>"
        'word/numbering.xml'="<w:numbering xmlns:w='$word'><w:abstractNum w:abstractNumId='1'><w:lvl w:ilvl='0'><w:start w:val='3'/><w:numFmt w:val='decimal'/><w:lvlText w:val='%1.'/></w:lvl></w:abstractNum><w:num w:numId='1'><w:abstractNumId w:val='1'/></w:num></w:numbering>"
        'word/_rels/document.xml.rels'="<Relationships xmlns='$relations'><Relationship Id='r1' Type='${office}styles' Target='styles.xml'/><Relationship Id='r2' Type='${office}numbering' Target='numbering.xml'/></Relationships>"
    }
}
function Archive($Parts,[switch]$Duplicate){
    $path=Join-Path $root ([Guid]::NewGuid().ToString('N')+'.docx')
    $zip=[IO.Compression.ZipFile]::Open($path,[IO.Compression.ZipArchiveMode]::Create)
    try{
        foreach($name in $Parts.Keys){
            $entry=$zip.CreateEntry($name)
            $stream=$entry.Open();$bytes=[Text.Encoding]::UTF8.GetBytes($Parts[$name])
            try{$stream.Write($bytes,0,$bytes.Length)}finally{$stream.Dispose()}
        }
        if($Duplicate){$zip.CreateEntry('word/document.xml')|Out-Null}
    }finally{$zip.Dispose()}
    $path
}
function Verify([string]$Path){
    Assert-ChatGptDocxExportFile $Path (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() $fixture
}
$source=[Text.UTF8Encoding]::new($false,$true).GetString([Convert]::FromBase64String($fixture.source_base64))
Assert ($source.StartsWith('# ELON_TEXT_BLOCK_ACCEPTANCE_V1') -and $source.Contains("`tsecond")) 'controlled_unicode_source'
$receipt=[pscustomobject]@{exported=$true;extension='docx';source_sha256=$fixture.source_sha256}
Assert-ChatGptDocxExportReceipt $receipt $fixture
$passed++
foreach($change in @(
    {param($r)$r.extension='txt'}, {param($r)$r.exported=$false},
    {param($r)$r.exported='true'}, {param($r)$r.source_sha256='0'*64})){
    $bad=$receipt|ConvertTo-Json|ConvertFrom-Json
    & $change $bad
    Reject {Assert-ChatGptDocxExportReceipt $bad $fixture} 'receipt_rejects_wrong_result'
}
$valid=Archive (PackageParts)
$result=Verify $valid
Assert ($result.package_valid -and $result.content_matches -and $result.paragraphs -eq 10 -and
    $result.tables -eq 1 -and $result.list_items -eq 2) 'complete_sample'
Assert ($result.PSObject.Properties.Name -notcontains 'content' -and $result.PSObject.Properties.Name -notcontains 'paragraph_text') 'summary_without_body'
Reject {Assert-ChatGptDocxExportFile $valid ('0'*64) $fixture} 'file_hash_checked'
$case=0
foreach($change in @(
    {param($p)$p['word/document.xml']=$p['word/document.xml'].Replace('ELON_LOCAL_EDIT_V1','stale')},
    {param($p)$p['word/document.xml']=$p['word/document.xml'].Replace('  first','first')},
    {param($p)$p['word/document.xml']=$p['word/document.xml'].Replace('w:tab','w:br')},
    {param($p)$p['word/document.xml']=$p['word/document.xml'].Replace('Heading1','Heading2')},
    {param($p)$p['word/document.xml']=$p['word/document.xml'].Replace('<w:b />','<w:i />')},
    {param($p)$p['word/document.xml']=$p['word/document.xml'].Replace('w:tbl','w:other')},
    {param($p)$p['word/numbering.xml']=$p['word/numbering.xml'].Replace("w:val='3'","w:val='1'")},
    {param($p)$p['word/document.xml']=$p['word/document.xml'].Replace('w:val="1"','w:val="99"')},
    {param($p)$p['word/numbering.xml']=$p['word/numbering.xml'].Replace("w:abstractNumId w:val='1'","w:abstractNumId w:val='99'")},
    {param($p)$p['word/numbering.xml']=$p['word/numbering.xml'].Replace("w:val='decimal'","w:val='bullet'")},
    {param($p)$p['word/styles.xml']=$p['word/styles.xml'].Replace('Heading1','Heading2')},
    {param($p)$p['word/styles.xml']='<wrong/>'},
    {param($p)$p['[Content_Types].xml']=$p['[Content_Types].xml'].Replace('application/xml','text/plain')},
    {param($p)$p['word/_rels/document.xml.rels']=$p['word/_rels/document.xml.rels'].Replace("Target='styles.xml'","Target='styles.xml' TargetMode='External'")},
    {param($p)$p['_rels/.rels']=$p['_rels/.rels'].Replace('word/document.xml','https://example.invalid/document.xml')},
    {param($p)$p['[Content_Types].xml']=$p['[Content_Types].xml'].Replace('document.main+xml','macroEnabled.main+xml')},
    {param($p)$p['word/styles.xml']="<!DOCTYPE styles [<!ENTITY x SYSTEM 'https://example.invalid/'>]><styles>&x;</styles>"},
    {param($p)$p['word/document.xml']='<broken'},
    {param($p)$p.Remove('word/styles.xml')},
    {param($p)$p['../unexpected.xml']='<unknown/>'},
    {param($p)$p['word/styles.xml']='x'*524289})){
    $case++;$parts=PackageParts;$beforeParts=$parts|ConvertTo-Json -Compress
    & $change $parts
    Assert (($parts|ConvertTo-Json -Compress) -cne $beforeParts) "mutation_${case}_applied"
    $invalid=Archive $parts
    Reject {Verify $invalid} "damaged_or_unsafe_document_$case"
}
Reject {Verify (Archive (PackageParts) -Duplicate)} 'duplicate_parts'
$plain=Join-Path $root 'renamed.docx'
[IO.File]::WriteAllText($plain,$source)
Reject {Verify $plain} 'renamed_markdown'
$large=Join-Path $root 'large.docx'
[IO.File]::WriteAllBytes($large,[byte[]]::new(1048577))
Reject {Verify $large} 'bounded_file'
$java=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'android/TextBlockUiAcceptance.java') -Raw
Assert ($java.Contains('md|py|txt|docx') -and $java.Contains('requested_export_format_missing') -and
    -not $java.Contains('extension = "txt";')) 'exact_native_format_no_substitution'
Assert ($java.Contains('edit_docx_fixture') -and $java.Contains('source_sha256') -and
    $java.Contains('if (!extension.equals("docx"))')) 'source_hash_not_zip_hash'
$smoke=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'smoke-chatgpt-web-text-block-ui.ps1') -Raw
Assert ($smoke.IndexOf("throw 'docx_requires_existing_writing_fixture'") -lt $smoke.IndexOf('New-ChatGptWebSmokeRuntime') -and
    $smoke.Contains("throw 'fixture_creation_required'") -and $smoke.Contains("throw 'idle_native_voice_required'")) 'fixture_and_idle_guards'
foreach($file in @('smoke-chatgpt-web-text-block-ui.ps1','chatgpt-text-block-docx-evidence.ps1')){
    $errors=$null;$tokens=$null
    [Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot $file),[ref]$tokens,[ref]$errors)|Out-Null
    Assert ($errors.Count -eq 0) 'script_syntax'
}
Write-Host "DOCX_ACCEPTANCE_EVIDENCE_TESTS=passed checks=$passed device_tested=false"
