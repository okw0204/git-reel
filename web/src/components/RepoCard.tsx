import { ExternalLink, GitFork, Star } from "lucide-react";
import type { Repository } from "../types";

type RepoCardProps = {
  repository: Repository;
};

export function RepoCard({ repository }: RepoCardProps) {
  return (
    <article className="repo-card">
      <div className="repo-card-header">
        <div>
          <p className="repo-owner">{repository.owner}</p>
          <h1>{repository.full_name}</h1>
        </div>
        <a className="icon-link" href={repository.html_url} target="_blank" rel="noreferrer" aria-label="GitHubで開く">
          <ExternalLink aria-hidden="true" size={20} />
        </a>
      </div>
      <p className="repo-description">{repository.description ?? "説明はまだありません"}</p>
      <div className="repo-meta" aria-label="リポジトリ情報">
        {repository.primary_language ? <span>{repository.primary_language}</span> : null}
        <span><Star aria-hidden="true" size={16} />{repository.stars.toLocaleString()}</span>
        <span><GitFork aria-hidden="true" size={16} />{repository.forks.toLocaleString()}</span>
        {repository.license ? <span>{repository.license}</span> : null}
      </div>
      <div className="topic-list">
        {repository.topics.map((topic) => (
          <span key={topic}>{topic}</span>
        ))}
      </div>
    </article>
  );
}
