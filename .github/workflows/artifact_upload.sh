artifact="$1"
artifact_track="$2"
file_path="$3"

file_name=$(basename -- "$file_path")

git_commit_shash=$(git rev-parse --short "$GITHUB_SHA")
git_branch=$(echo $GITHUB_REF | cut -d'/' -f 3)

version="$ARTIFACT_VERSION"
if [[ -z "$ARTIFACT_VERSION" ]]; then
    echo "Missing ARTIFACT_VERSION env var"
    exit 1
fi

echo "Uploading $file_path"
curl -H "Content-Type:multipart/form-data" \
    -X POST \
    -F "info={\"version\": \"$version\", \"versionHash\": \"$git_commit_shash\", \"updateLatest\": true }" \
    -F "payload=@$file_path; filename=${artifact}_${git_commit_shash}.${file_name##*.}" \
    "https://valth.run/api/artifacts/$artifact/$artifact_track?api-key=$ARTIFACT_API_KEY" || exit 1